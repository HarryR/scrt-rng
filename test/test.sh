#!/bin/bash 

function tx_wait () {
	local txid=$1
	echo -n " $txid "
	until res=`secretd query tx $txid 2> /dev/null`; do
		echo -n '.'
		sleep 1
	done
	echo ""
}

CODE_WASM=/src/target/wasm32-unknown-unknown/release/scrt_rng.wasm
CODE_HASH=`sha256sum $CODE_WASM | cut -f 1 -d ' ' | tr 'a-z' 'A-Z'`

echo "Code hash: $CODE_HASH"

# Deploy contract code file
echo -n "Uploading code"
DEPLOY_TX=`secretd tx compute store -y --from a --gas 1000000 $CODE_WASM.gz | jq -r '.txhash'`
tx_wait $DEPLOY_TX

# Find ID of contract deployed with the same code hash
CODE_ID=`secretd query compute list-code | jq -r 'map(select(.data_hash = "$CODE_HASH")) | .[-1].id'`

# Submit initialization transaction
echo -n "Init contract"
TX_INIT=`secretd tx compute instantiate $CODE_ID "{}" --from a --label rng-init$CODE_ID -y | jq -er '.txhash'`
tx_wait $TX_INIT

# Retrieve deployed contract address
CONTRACT=`secretd query compute tx "$TX_INIT" | jq -er '.output_logs[0].attributes[0].value'`

# Submit entropy to contract
echo -n "Seed contract"
SEED1=`openssl rand -hex 64`
TX_SEED1=`secretd tx compute execute $CONTRACT '{"donate":{"entropy":"$SEED1"}}' -y --from a --label rng-seed$CODE_ID | jq -er '.txhash'`
tx_wait $TX_SEED1
