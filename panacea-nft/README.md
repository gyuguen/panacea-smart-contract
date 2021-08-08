# Panacea NFT

## Required
* panacea-core
* Rust
* Docker
* jq

## Install

### panacea-nft
You must go to the panacea-nft path and compile it.
```shell
cd panacea-nft
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.11.5
cd artifacts/
```

You can check the two files below.
```shell
checksums.txt    panacea_nft.wasm
```

Store this contract in panacea.
```shell
TX_FLAG=(--chain-id {your_chain} --fees 1000000umed --gas 3000000)
MINTER=$(panacead keys show {your address or key} -a)
STORE_RES=$(panacead tx wasm store panacea_nft.wasm --from $MINTER $TX_FLAG -y)
echo $STORE_RES | jq
NFT_CODE=$(echo $STORE_RES | jq -r '.logs[].events[].attributes[] | select(.key == "code_id") | .value')
echo $NFT_CODE
```
Instantiate contract
```shell
INIT=$(jq -n --arg name "panacea nft" --arg symbol "p_nft" --arg minter $MINTER '{"name":$name,"symbol":$symbol,"minter":$minter}')

INIT_RES=$(panacead tx wasm instantiate $NFT_CODE "$INIT" \
--label 'panacea-nft' \
--from $MINTER \
$TX_FLAG -y)
echo $INIT_RES | jq

NFT_CONTRACT=$(echo $INIT_RES | jq -r '.logs[].events[].attributes[0] | select(.key == "contract_address").value')
echo $NFT_CONTRACT
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
echo $TRANSFER_OWNER
TRANSFER=$(jq -n --arg recipient $TRANSFER_OWNER --arg token_id $TOKEN_ID '{"transfer_nft":{"recipient":$recipient, "token_id":$token_id}}')
TRANSFER_RES=$(panacead tx wasm execute $NFT_CONTRACT $TRANSFER --from $MINTER $TX_FLAG -y)
echo $TRANSFER_RES | jq
# Get owner info
QUERY_OWNER_OF=$(jq -n --arg token_id $TOKEN_ID '{"owner_of":{"token_id":$token_id}}')
panacead q wasm contract-state smart $NFT_CONTRACT $QUERY_OWNER_OF # Changed transfer_owner
```