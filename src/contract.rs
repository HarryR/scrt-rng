use std::option::Option;

use secret_cosmwasm_std::{Storage, Api, Querier, InitResponse, StdError,
                          StdResult, Extern, Env, WasmMsg, to_binary,
                          HumanAddr, HandleResponse, QueryResult};

use serde::{Deserialize, Serialize};

use schemars::JsonSchema;

const SEED_KEY: &[u8] = b"seed";


// -----------------------------------------------------------------


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RemoteContract
{
    ReceiveRandom {
        rn: [u8; 32],
        cb_msg: String },
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg { }


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg
{
    Callback {
        cb_msg: String,
        callback_code_hash: String,
        contract_addr: HumanAddr },

    Donate { entropy: String },
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg { }


// -----------------------------------------------------------------


#[cfg(feature="rng_blake2")]
mod rng
{
    use std::option::Option;
    use secret_cosmwasm_std::Env;

    use blake2::{Blake2s256, Digest};

    pub fn cycle (seed : &[u8], env: &Env, entropy: &Option<String>) -> [u8; 32]
    {
        let mut st = Blake2s256::new();

        st.update(&seed);

        if let Some(x) = &entropy
        {
            st.update(x.as_bytes());
        }

        st.update(&env.block.height.to_le_bytes());

        st.update(&env.block.time.to_le_bytes());

        st.update(&env.block.chain_id.as_bytes());

        st.update(&env.message.sender.as_str().as_bytes());

        st.update(&env.contract.address.as_str().as_bytes());

        st.update(&env.contract_code_hash.as_bytes());

        st.finalize().into()
    }

    pub fn emit (seed: &[u8; 32]) -> [u8; 32]
    {
        let mut st = Blake2s256::new();

        st.update(&seed);

        st.finalize().into()
    }
}


// -----------------------------------------------------------------


// https://csrc.nist.gov/CSRC/media/Projects/Lightweight-Cryptography/documents/round-1/spec-doc/Xoodyak-spec.pdf
#[cfg(feature="rng_xood")]
mod rng
{
    use std::option::Option;
    use secret_cosmwasm_std::Env;

    use xoodyak::{XoodyakHash, XoodyakCommon};

    pub fn cycle (seed : &[u8], env: &Env, entropy: &Option<String>) -> [u8; 32]
    {
        let mut st = XoodyakHash::new();

        st.absorb(seed);

        if let Some(x) = &entropy
        {
            st.absorb(x.as_bytes());
        }

        // See: https://docs.scrt.network/dev/privacy-model-of-secret-contracts.html#init-and-handle

        st.absorb(&env.block.height.to_le_bytes());

        st.absorb(&env.block.time.to_le_bytes());

        st.absorb(env.block.chain_id.as_bytes());

        st.absorb(env.message.sender.as_str().as_bytes());

        st.absorb(env.contract.address.as_str().as_bytes());

        st.absorb(env.contract_code_hash.as_bytes());

        let mut out = [0u8; 32];

        st.squeeze_key(&mut out);

        out
    }

    pub fn emit (seed: &[u8; 32]) -> [u8; 32]
    {
        let mut dest = [0u8; 32];

        let mut st = XoodyakHash::new();

        st.absorb(&seed[..]);

        st.squeeze_key(&mut dest);

        dest
    }
}


// -----------------------------------------------------------------


// Note: https://www.johndcook.com/blog/2020/02/22/chacha-rng-with-fewer-rounds/
#[cfg(feature="rng_sha256chacha20")]
mod rng
{
    use std::option::Option;
    use secret_cosmwasm_std::Env;

    use rand_chacha::ChaCha20Rng;
    use rand_chacha::rand_core::{SeedableRng, RngCore};
    use sha2::{Sha256, Digest};

    pub fn cycle (seed : &[u8], env: &Env, entropy: &Option<String>) -> [u8; 32]
    {
        let mut st = Sha256::new();

        st.update(&seed);

        if let Some(x) = &entropy
        {
            st.update(x.as_bytes());
        }

        st.update(&env.block.height.to_le_bytes());

        st.update(&env.block.time.to_le_bytes());

        st.update(&env.block.chain_id.as_bytes());

        st.update(&env.message.sender.as_str().as_bytes());

        st.update(&env.contract.address.as_str().as_bytes());

        st.update(&env.contract_code_hash.as_bytes());

        st.finalize().into()
    }

    pub fn emit (seed: &[u8; 32]) -> [u8; 32]
    {
        let mut dest = [0u8; 32];

        ChaCha20Rng::from_seed(*seed).fill_bytes(&mut dest[..]);

        dest
    }
}


// -----------------------------------------------------------------


fn cycle_seed<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: &Env,
    entropy: &Option<String>,
) -> StdResult<[u8; 32]>
{
    let seed = deps.storage.get(SEED_KEY).ok_or_else(|| StdError::not_found("seed"))?;

    let out = rng::cycle(&seed, env, entropy);

    deps.storage.set(SEED_KEY, &out);

    Ok(out)
}


fn get_rn<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: &Env,
    entropy: &Option<String>,
) -> StdResult<[u8; 32]>
{
    let dest = rng::emit(&cycle_seed(deps, env, entropy)?);

    Ok(dest)
}


// -----------------------------------------------------------------


fn handle_callback<S: Storage, A: Api, Q: Querier>( 
    deps: &mut Extern<S, A, Q>,
    env: Env,
    cb_msg: String,
    contract_addr: HumanAddr,
    callback_code_hash: String,
) -> StdResult<HandleResponse>
{
    let msg = to_binary(&RemoteContract::ReceiveRandom {
        rn: get_rn(deps, &env, &None)?,
        cb_msg
    })?;

    let send = Vec::new();

    Ok(HandleResponse {
        messages: vec![ WasmMsg::Execute {
                            msg, contract_addr, callback_code_hash, send
                        }.into() ],
        log: vec![],
        data: None
    })
}


fn handle_donate<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    entropy: String
) -> StdResult<HandleResponse>
{
    cycle_seed(deps, &env, &Some(entropy))?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: None
    })
}


// -----------------------------------------------------------------


pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: InitMsg
) -> StdResult<InitResponse>
{
    let seed = [0u8; 32];   // contract is always deployed in a consistent state

    deps.storage.set(SEED_KEY, &seed);

    Ok(InitResponse::default())
}


pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult <HandleResponse>
{
    // Prevent lost funds
    if !env.message.sent_funds.is_empty()
    {
        return Err(StdError::unauthorized());
    }

    match msg
    {
        HandleMsg::Callback {
            cb_msg, callback_code_hash, contract_addr
        } => handle_callback(deps, env, cb_msg, contract_addr, callback_code_hash),

        HandleMsg::Donate { entropy } => handle_donate(deps, env, entropy),
    }
}


pub fn query<S: Storage, A: Api, Q: Querier>(
    _deps: &Extern<S, A, Q>,
    _msg: QueryMsg,
) -> QueryResult
{
    Err(StdError::unauthorized())
}
