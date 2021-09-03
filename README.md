# Panacea Smart Contract

## Prerequisites
* panacea-core
* Docker
* Rust
* jq

## Introduction
This document covers:
- How to create Panacea NFT contracts for minting/transferring NFTs.
- How to create Panacea NFT Redeem contracts for exchanging NFTs for MED.

## Creating a NFT contract
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
TX_FLAG=(--chain-id {your chainID} --gas auto --gas-prices 5umed --gas-adjustment 1.3)
MINTER=$(panacead keys show {your address or key} -a)
RES=$(panacead tx wasm store panacea_nft.wasm --from $MINTER $TX_FLAG -y) # 10MED used fee...
NFT_CODE=$(echo $RES | jq -r '.logs[].events[].attributes[] | select(.key == "code_id") | .value')
echo $NFT_CODE
```
Instantiate contract
```shell
INIT=$(jq -n --arg name "panacea nft" --arg symbol "p_nft" --arg minter $MINTER '{"name":$name,"symbol":$symbol,"minter":$minter}')

INIT_RES=$(panacead tx wasm instantiate $NFT_CODE "$INIT" \
--label 'panacea-nft' \
--from $MINTER \
$TX_FLAG -y) # 0.8MED used fee

NFT_CONTRACT=$(echo $INIT_RES | jq -r '.logs[].events[].attributes[0] | select(.key == "contract_address").value')
echo $NFT_CONTRACT
```

Mint NFT
```shell
MINT=$(jq -n --arg owner $MINTER --arg name "panacea_nft_1" --arg denom "umed" --arg amount "1000000000" '{"mint":{"owner":$owner, "name":$name, "price":{"denom":$denom, "amount":$amount}}}')
MINT_RES=$(panacead tx wasm execute $NFT_CONTRACT $MINT --from $MINTER $TX_FLAG -y)
TOKEN_ID=$(echo $MINT_RES | jq -r '.logs[].events[].attributes[] | select(.key == "token_id")'.value) # 0.8MED used fee
echo $TOKEN_ID
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

Transfer ownership of the NFT
```shell
# make transfer_owner key
panacead keys add transfer_owner
TRANSFER_OWNER=$(panacead keys show transfer_owner -a)
panacead tx bank send $MINTER $TRANSFER_OWNER 2000000umed --from $MINTER $TX_FLAG -y
echo $MINTER
echo $TRANSFER_OWNER

TRANSFER=$(jq -n --arg recipient $TRANSFER_OWNER --arg token_id $TOKEN_ID '{"transfer_nft":{"recipient":$recipient, "token_id":$token_id}}')
TRANSFER_RES=$(panacead tx wasm execute $NFT_CONTRACT $TRANSFER --from $MINTER $TX_FLAG -y)
echo $TRANSFER_RES | jq

# Get owner info
QUERY_OWNER_OF=$(jq -n --arg token_id $TOKEN_ID '{"owner_of":{"token_id":$token_id}}')
panacead q wasm contract-state smart $NFT_CONTRACT $QUERY_OWNER_OF # Changed transfer_owner
echo $TRANSFER_OWNER
```

## Creating Panacea NFT Redeem contract
You must go to the panacea-nft-redeem path and compile it.
```shell
cd panacea-nft-redeem
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.11.5
cd artifacts/
```

You will see that two files were generated as below.
```shell
checksums.txt    panacea_nft_redeem.wasm
```

Store this contract in panacea.
```shell
TX_FLAG=(--chain-id {your chainID} --gas auto --gas-prices 5umed --gas-adjustment 1.3)
CREATOR=$(panacead keys show {your address or key} -a)
echo $CREATOR
STORE_RES=$(panacead tx wasm store payment_guarantee.wasm --from $CREATOR $TX_FLAG -y) # 6.5MED used fee
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
--label 'payments guarantee') #0.8MED used fee

PAYMENT_CONTRACT=$(echo $INIT_RES | jq -r '.logs[].events[].attributes[0] | select(.key == "contract_address").value')
echo $PAYMENT_CONTRACT
```

You have to put the deposit in contract.<br/>
You can add the deposit as below or directly to the contract address.

```shell
DEPOSIT_RES=$(panacead tx wasm execute $PAYMENT_CONTRACT '{"deposit":{}}' --amount 1000000000umed --from $CREATOR $TX_FLAG -y)
# Get balances
panacead q bank balances $PAYMENT_CONTRACT
```

## NFT Transactions (Reward payments)
In order to receive the reward, the NFT must be sent (returned) to the `payment-guarantee` contract. For that, the contract address and token_id are required.
If the transaction is successful, the owner of the NFT will be changed and the amount specified in the NFT will be paid to the NFT exchange requester.
(Unimplemented) If the transaction fails, the owner of the NFT becomes the exchange requester.
```shell
# before
panacead q bank balances $TRANSFER_OWNER # Deposit amount excluding fees
REWARD_NFT=$(jq -n --arg contract $PAYMENT_CONTRACT --arg token_id $TOKEN_ID '{"send_nft":{"contract":$contract,"token_id":$token_id}}')
echo $REWARD_NFT
REWARD_NFT_RES=$(panacead tx wasm execute $NFT_CONTRACT "$REWARD_NFT" --from $TRANSFER_OWNER $TX_FLAG -y)
# after
QUERY_OWNER_OF=$(jq -n --arg token_id $TOKEN_ID '{"owner_of":{"token_id":$token_id}}')
panacead q wasm contract-state smart $NFT_CONTRACT $QUERY_OWNER_OF # owner is 'creator'
panacead q bank balances $PAYMENT_CONTRACT
panacead q bank balances $TRANSFER_OWNER # Deposit amount excluding fees
```
