ABI files generated from https://github.com/ethereum-attestation-service/eas-contracts using https://github.com/foundry-rs/starknet-foundry.


```
# prepare the code
git clone https://github.com/ethereum-attestation-service/eas-contracts
cd eas-contracts
git checkout e16a73137a9c8039e97938a9f34c5eb6cbdafb70 # compatible updates on sepolia, mainnet and optimism

forge build
forge bind


# copy generated go files
cp -r bindings  $PATH_TO_RUST_PROJECTS/eas-sdk-rs/.
``` 