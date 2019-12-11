# biboy

Bare-metal gameboy for BIOS machines.

1. Setup

```
./build.sh setup
```

2. Build

```
./build.sh
```

3. Run

```
./build.sh run
```

## Configuration

`vars.sh` contains configuration as environment vairables.

```sh
export BIBOY_FREQ=4194300
export BIBOY_SAMPLE=4194
export BIBOY_DELAY_UNIT=10
export BIBOY_NATIVE=false

# Here specify your ROM file
export BIBOY_ROM="$(pwd)/../gbr/core/Kirby's Dream Land (USA, Europe).gb"
```
