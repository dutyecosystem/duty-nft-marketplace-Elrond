erdpy contract build

# deploy by project 

#erdpy --verbose contract deploy --chain="D" --project=nft-marketplace --pem="../wallet-owner.pem" --gas-limit=80000000 --proxy="https://devnet-gateway.elrond.com" --recall-nonce --send --arguments "1000"

# deploy by wasm 

erdpy --verbose contract deploy --chain="D" --bytecode "output/nft-marketplace.wasm" --pem="../../wallet-owner.pem" --gas-limit=80000000 --proxy="https://devnet-gateway.elrond.com" --recall-nonce --send --arguments "1000"

erdpy --verbose contract deploy --chain="D" --bytecode "output/reward-handler.wasm" --pem="../../wallet-owner.pem" --gas-limit=600000000 --proxy="https://devnet-gateway.elrond.com" --recall-nonce --send --arguments "erd1qqqqqqqqqqqqqpgqketgs96wx6yp652luc539yaccm9ac4r8uugqm828hu" 

erdpy --verbose contract deploy --chain="D" --bytecode "output/nft-collection.wasm" --pem="../../wallet-owner.pem" --gas-limit=600000000 --proxy="https://devnet-gateway.elrond.com" --recall-nonce --send --arguments "erd1p467yn4jzn88le3m2drlsqynk6nesn5l8dd08nuxk6qy4mvduugqqkmmpz" "erd1p467yn4jzn88le3m2drlsqynk6nesn5l8dd08nuxk6qy4mvduugqqkmmpz"

# erdpy contract test

# erdpy contract test --wildcard view_functions.scen.json

# cargo test

# nft-collection erd1qqqqqqqqqqqqqpgqketgs96wx6yp652luc539yaccm9ac4r8uugqm828hu

# nft-marketplace erd1qqqqqqqqqqqqqpgqgcf6pxk7vgwjsahcevjl9phdectnfrrhuugq6kz8ua

# reward-handler erd1qqqqqqqqqqqqqpgqv0e6wf068mz8v0j8xw2wj62d9acv2e5luugqy27yj4