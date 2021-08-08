# Payment Guarantee

## Required
* Docker

## Install

### payment guarantee
You must go to the payment-guarantee path and compile it.
```shell
cd payment-guarantee
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.11.5
cd artifacts/
```

You can check the two files below.
```shell
checksums.txt    payment_guarantee.wasm
```

Store this contract in panacea.
```shell
TX_FLAG=(--chain-id {your_chain} --fees 1000000umed --gas auto)
CREATOR=$(panacead keys show {your address or key} -a)
STORE_RES=$(panacead tx wasm store payment_guarantee.wasm --from $CREATOR $TX_FLAG -y)
PAYMENT_CODE=$(echo $STORE_RES | jq -r '.logs[].events[].attributes[] | select(.key == "code_id") | .value')
```
Instantiate contract
```shell
PAYMENT_INIT=$(jq -n --arg contracts "$NFT_CONTRACT" '{"source_contracts":$contracts | split(" ")}')

INIT_RES=$(panacead tx wasm instantiate $PAYMENT_CODE "$PAYMENT_INIT" \
--from $CREATOR \
$TX_FLAG -y \
--label 'payments guarantee')

PAYMENT_CONTRACT=$(echo $INIT_RES | jq -r '.logs[].events[].attributes[0] | select(.key == "contract_address").value')
```

Deposit
```shell
DEPOSIT_RES=$(panacead tx wasm execute $PAYMENT_CONTRACT '{"deposit":{}}' --amount 1000000000umed --from $CREATOR $TX_FLAG -y)

# Get balances
panacead q bank balances $PAYMENT_CONTRACT
```