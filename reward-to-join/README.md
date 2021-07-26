# panacea-smart-contract

## Requirement
* Go (v1.15 ~)
* Rust 
* Docker

## Build
### Install wasm32
```shell
rustup default stable
cargo version
# If this is lower than 1.50.0+, update
rustup update stable

rustup target list --installed
rustup target add wasm32-unknown-unknown
```

### Source Checkout
```shell
git clone https://github.com/gyuguen/panacea-smart-contract
cd panacea-smart-contract
```

### Unit test
```shell
RUST_BACKTRACE=1 cargo unit-test
```

### Compile
```shell
RUST_FLAGS='-C link-arg=-s' cargo panacea
```

### Optimized Compilation
```shell
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.11.3

```

## Deploy
### Store smart contract in panacea
```shell
PAYER={bech32:address}
JOINER={bech32:address}
TXFLAG=(--chain-id {your_chain_id} --gas auto --fees 1000000umed)
RES=$(panacead tx wasm store artifacts/panacea_smart_contract.wasm --from $PAYER $TXFLAG -y)
CODE_ID=$(echo $RES | jq -r '.logs[0].events[0].attributes[-1].value')

panacead query wasm code $CODE_ID download.wasm

# Same
diff artifacts/panacea_smart_contract.wasm download.wasm
```

## 작업 시나리오
* 지불자(Payer)와 가입자(Joiner) 존재
* 지불자가 계약 생성
* 계약 조건은 진료 건수 1000건 이상, 보험 청구 진행
* 계약 조건이 충족되면 가입자에게 보상 지급
* 계약 조건이 충족되지 못하면 보상 지급되지 않음

### Instantiating Contract
```shell
INIT=$(jq -n --arg joiner $JOINER '{"joiner":$joiner,"term_of_payments":[{"id":"id","contract_content":{"treatments":100,"insurance_claim":true,"period_days":0},"amount":{"amount":"200000000000","denom":"umed"},"is_payment":false}]}')
panacead tx wasm instantiate $CODE_ID "$INIT" \
--from $PAYER --amount 100000000000umed --label "panacea Contract" $TXFLAG -y

CONTRACT=$(panacead query wasm list-contract-by-code $CODE_ID --output json | jq -r '.contracts[-1]')
echo $CONTRACT

panacead q wasm contract $CONTRACT
panacead q bank balances $CONTRACT
panacead q wasm contract-state all $CONTRACT --output json | jq -r '.models[0].value' | base64 --decode

# 보상 지급 조건이 만족하지 않은 경우 에러 발생
APPROVE='{"approve":{}}'
# 에러 발생
panacead tx wasm execute $CONTRACT $APPROVE --from $JOINER $TXFLAG
```

### Update achievement
```shell
UPDATE='{"update":{"treatments":1001,"insurance_claim":true}}'
panacead tx wasm execute $CONTRACT $UPDATE --from gyuguen $TXFLAG
panacead q wasm contract-state all $CONTRACT --output json | jq -r '.models[0].value' | base64 --decode
```

### Reward receipt
```shell
# 성공
panacead tx wasm execute $CONTRACT $APPROVE --from $JOINER $TXFLAG

panacead q bank balances $JOINER
```

