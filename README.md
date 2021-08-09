# Panacea Smart Contract

## Required
* Docker

## This document
How to create a Payment Garantee contract with NFT and exchange tokens.

## Creating NFT
You must go to the panacea-nft directory and compile it.
```shell
cd panacea-nft
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.11.5
cd artifacts/
```

You will see that two files were generated as below.
```shell
checksums.txt    panacea_nft.wasm
```

Store this contract in panacea.
```shell
TX_FLAG=(--chain-id {your_chain} --fees 1000000umed --gas auto)
MINTER=$(panacead keys show {your address or key} -a)
RES=$(panacead tx wasm store panacea_nft.wasm --from $MINTER $TX_FLAG -y)
NFT_CODE=$(echo $RES | jq -r '.logs[].events[].attributes[] | select(.key == "code_id") | .value')
```
Instantiate contract
```shell
INIT=$(jq -n --arg name "panacea nft" --arg symbol "p_nft" --arg minter $MINTER '{"name":$name,"symbol":$symbol,"minter":$minter}')

INIT_RES=$(panacead tx wasm instantiate $NFT_CODE "$INIT" \
--label 'panacea-nft' \
--from $MINTER \
$TX_FLAG -y)

NFT_CONTRACT=$(echo $INIT_RES | jq -r '.logs[].events[].attributes[0] | select(.key == "contract_address").value')
```

Mint NFT
```shell
MINT=$(jq -n --arg owner $MINTER --arg name "panacea_nft_1" --arg denom "umed" --arg amount "1000000000" '{"mint":{"owner":$owner, "name":$name, "price":{"denom":$denom, "amount":$amount}}}')
MINT_RES=$(panacead tx wasm execute $NFT_CONTRACT $MINT --from $MINTER $TX_FLAG -y)
TOKEN_ID=$(echo $MINT_RES | jq -r '.logs[].events[].attributes[] | select(.key == "token_id")'.value)
# Get contract info
QUERY_CONTRACT_INFO='{"contract_info":{}}'
panacead q wasm contract-state smart $NFT_CONTRACT $QUERY_CONTRACT_INFO
# Get token info
QUERY_TOKEN_INFO=$(jq -n --arg token_id $TOKEN_ID '{"nft_info":{"token_id":$token_id}}')
panacead q wasm contract-state smart $NFT_CONTRACT $QUERY_TOKEN_INFO
# Get owner info
QUERY_OWNER_OF=$(jq -n --arg token_id $TOKEN_ID '{"owner_of":{"token_id":$token_id}}')
panacead q wasm contract-state smart $NFT_CONTRACT $QUERY_OWNER_OF
```

Transfer owner of NFT
```shell
TRANSFER_OWNER=$(panacead keys show {key or address} -a)
TRANSFER=$(jq -n --arg recipient $TRANSFER_OWNER --arg token_id $TOKEN_ID '{"transfer_nft":{"recipient":$recipient, "token_id":$token_id}}')
TRANSFER_RES=$(panacead tx wasm execute $NFT_CONTRACT $TRANSFER --from $MINTER $TX_FLAG -y)
echo $TRANSFER_RES | jq
# Get owner info
QUERY_OWNER_OF=$(jq -n --arg token_id $TOKEN_ID '{"owner_of":{"token_id":$token_id}}')
panacead q wasm contract-state smart $NFT_CONTRACT $QUERY_OWNER_OF # Changed transfer_owner
```

## Create Payment Guarantee contract
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
TX_FLAG=(--chain-id {your_chain} --fees 1000000umed --gas 300000)
CREATOR=$(panacead keys show {your address or key} -a)
STORE_RES=$(panacead tx wasm store payment_guarantee.wasm --from $CREATOR $TX_FLAG -y)
echo $STORE_RES | jq
PAYMENT_CODE=$(echo $STORE_RES | jq -r '.logs[].events[].attributes[] | select(.key == "code_id") | .value')
echo $PAYMENT_CODE
```
Instantiate contract
```shell
PAYMENT_INIT=$(jq -n --arg contracts "$NFT_CONTRACT" '{"source_contracts":$contracts | split(" ")}')

INIT_RES=$(panacead tx wasm instantiate $PAYMENT_CODE "$PAYMENT_INIT" \
--from $CREATOR \
$TX_FLAG -y \
--label 'payments guarantee')

PAYMENT_CONTRACT=$(echo $INIT_RES | jq -r '.logs[].events[].attributes[0] | select(.key == "contract_address").value')
echo $PAYMENT_CONTRACT
```

You have to put the deposit in contract.<br/>
You can add the deposit as below or directly to the contract address.

```shell
DEPOSIT_RES=$(panacead tx wasm execute $PAYMENT_CONTRACT '{"deposit":{}}' --amount 1500000000umed --from $CREATOR $TX_FLAG -y)
# Get balances
panacead q bank balances $PAYMENT_CONTRACT
```

## NFT Transactions (Reward payments)
Payment-guarantee's contract account address and token_id are required to receive the Reward.<br/>
If the transaction is successful, the owner of the NFT will be changed and paid to the previous owner by the amount specified in the NFT.
```shell
REWARD_NFT=$(jq -n --arg contract $PAYMENT_CONTRACT --arg token_id $TOKEN_ID '{"send_nft":{"contract":$contract,"token_id":$token_id}}')
REWARD_NFT_RES=$(panacead tx wasm execute $NFT_CONTRACT "$REWARD_NFT" --from $TRANSFER_OWNER $TX_FLAG -y)
QUERY_OWNER_OF=$(jq -n --arg token_id $TOKEN_ID '{"owner_of":{"token_id":$token_id}}')
panacead q wasm contract-state smart $NFT_CONTRACT $QUERY_OWNER_OF # owner is 'creator'
panacead q bank balances $TRANSFER_OWNER # Deposit amount excluding fees
```
