# avail-light

[![Build status](https://github.com/maticnetwork/avail-light/actions/workflows/default.yml/badge.svg)](https://github.com/maticnetwork/avail-light/actions/workflows/default.yml) [![Code coverage](https://codecov.io/gh/maticnetwork/avail-light/branch/main/graph/badge.svg?token=7O2EA7QMC2)](https://codecov.io/gh/maticnetwork/avail-light)

Light client for Data Availability Blockchain of Polygon 💻

![demo](./img/prod_demo.png)

## Introduction

Naive approach for building one AVAIL light client, which will do following

- Listen for newly mined blocks
- As soon as new block is available, attempts to eventually gain confidence by asking for proof from full client _( via JSON RPC interface )_ for `N` many cells where cell is defined as `{row, col}` pair. The number of cells are randomly sampled to fetch the confidence eventually.

### Modes of Operation

1. **Light client**: This is the basic mode of operation and is always active in whichever mode it works. If the APP_ID is not given then this mode will be commence. In this mode, the client on each header it receieves will do the random sampling using RPC calls. The default number of random sampling is 8. It gets random cells in return, which verifies and calculate the confidence.
2. **App-Specific-Mode**: In this mode if an App_ID(greater than 0) is given in the config file then the client would find out the cols related the App_ID given using the app_data_lookup in the header. Then it fetches the 50% of cells from the cols, verifies them and uses them to decode the app_extrinsics_data. 
3. **Fat-Client-Mode**: In this mode, the client gets the entire extended matrix using IPFS if it is available or uses RPC calls to fetch. It verifies the entire cells and computes the CID mapping for the IPFS Pinning. It then decodes the entire matrix and reconstruct the entire app_specific_data related to all App_IDs.

## Installation

- First clone this repo in your local setup
- Create one yaml configuration file in root of project & put following content

```bash
touch config.yaml
```

```yaml
http_server_host = "127.0.0.1"
http_server_port = 7000

ipfs_seed = 1
ipfs_port = 37000
ipfs_path = "avail_ipfs_store"

# put full_node_rpc = https://devnet-avail.polygon.technology/ incase you are connecting to devnet
full_node_rpc = "http://127.0.0.1:9933"
# put full_node_ws = wss://devnet-avail.polygon.technology/ws incase you are connecting to devnet
full_node_ws = "ws://127.0.0.1:9944"
# None in case of default Light Client Mode
app_id = 0

confidence = 92.0
avail_path = "avail_path"

bootstraps = [["12D3KooWMm1c4pzeLPGkkCJMAgFbsfQ8xmVDusg272icWsaNHWzN", "/ip4/127.0.0.1/tcp/39000"]]

# See https://docs.rs/log/0.4.14/log/enum.LevelFilter.html for possible log level values
log_level = "INFO"
```

- Now, let's run client

```bash
cargo run -- -c config.yaml  
```

## Usage

Given block number ( as _(hexa-)_ decimal number ) returns confidence obtained by light client for this block

```bash
curl -s localhost:7000/v1/confidence/ _block-number_
```

```json
{
    "number": 223,
    "confidence": 99.90234375,
    "serialisedConfidence": "958776730446"
}
```

---

**Note :** Serialised confidence calculated as: 
> `blockNumber << 32 | int32(confidence * 10 ** 7)`, where confidence is represented as out of 10 ** 9

## Test code coverage report

We are using [grcov](https://github.com/mozilla/grcov) to aggregate code coverage information and generate reports.

To install grcov run

	$> cargo install grcov

Source code coverage data is generated when running tests with

	$> env RUSTFLAGS="-C instrument-coverage" \
		LLVM_PROFILE_FILE="tests-coverage-%p-%m.profraw" \
		cargo test

To generate report, run

	$> grcov . -s . \
		--binary-path ./target/debug/ \
		-t html \
		--branch \
		--ignore-not-existing -o \
		./target/debug/coverage/

To clean up generate coverage information files, run

	$> find . -name \*.profraw -type f -exec rm -f {} +

Open `index.html` from `./target/debug/coverage/` folder to review coverage data.


