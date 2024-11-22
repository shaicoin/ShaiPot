
# Shaipot - Shaicoin Miner

##


Welcome to **Shaipot**, a Shaicoin miner written in Rust. Shaipot is designed for efficiency and speed, supporting multi-threaded mining with minimal setup.

## Getting Started

To start mining with **Shaipot**, you need to provide the necessary arguments to connect to a mining pool and specify your Shaicoin address. Let's walk through how to set up and start mining.

### Required Arguments

- `--address <shaicoin_address>`  
  Your **Shaicoin address** where you want your mining rewards to be sent.
  
- `--pool <POOL_URL>`  
  The **pool URL** to which your miner will connect for jobs. This should be a valid WebSocket URL for the pool.

### Optional Arguments

- `--threads <AMT>`  
  Specifies the number of threads to use for mining. By default, the miner will automatically detect the optimal number of threads based on your system's available cores, but you can override this by specifying a value manually.

- `--vdftime <SECONDS>`  
  Specifies the number of seconds to wait before bailing out of the Hamiltonian graph search for mining. By default, the miner will automatically use 1 second. However, for slower CPUs this might need to be adjusted. 

## Compilation

To ensure **Shaipot** is compiled with the highest optimization for your CPU, use the following command:

```bash
cargo rustc --release -- -C opt-level=3 -C target-cpu=native -C codegen-units=1 -C debuginfo=0
```

This will optimize the build for your specific system, ensuring maximum performance during mining.

After compilation, the resulting executable will be located in the `target/release` directory. You can run it from there using the following command:

```bash
./target/release/shaipot --address <shaicoin_address> --pool <POOL_URL> [--threads <AMT>] [--vdftime <SECONDS>]
```

Make sure to replace `<shaicoin_address>` and `<POOL_URL>` with your actual Shaicoin address and the pool URL you're using.

## Running the Program

Once compiled, **Shaipot** is ready to run! Simply use the command provided above, specifying your Shaicoin address, the pool URL, and (optionally) the number of threads. Here's an example:

```bash
./target/release/shaipot --address sh1qeexkz69dz6j4q0zt0pkn36650yevwc8eksqeuu --pool wss://pool.shaicoin.org --threads 4
```

Example usage of vdftime looks like the following
```bash
--vdftime 1.5
```

This will start the mining process, and you'll see output as **Shaipot** connects to the pool and begins mining.

```plaintext
                          __
                         // \
                         \\_/ // 
    brrr''-.._.-''-.._.. -(||)(')
                         '''  
        _
     __( )_
    (      (o____
     |          |
     |      (__/
       \     /   ___
       /     \  \___/
     /    ^    /     \
    |   |  |__|_ SHA  |
    |    \______)____/
     \         /
       \     /_
        |  ( __)
        (____)
```

Happy Mining!
