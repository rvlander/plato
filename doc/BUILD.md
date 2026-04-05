# Build

Start by cloning the repository:

```sh
git clone https://github.com/baskerville/plato.git
cd plato
```

## Plato

#### Preliminary

Install the appropriate [compiler toolchain](https://drive.google.com/drive/folders/1YT6x2X070-cg_E8iWvNUUrWg5-t_YcV0) (the binaries of the `bin` directory need to be in your path).

Install the required dependencies: `wget`, `curl`, `git`, `pkg-config`, `unzip`, `jq`, `patchelf`.

Install _rustup_:

```sh
curl https://sh.rustup.rs -sSf | sh
```

Install the appropriate target:

```sh
rustup target add arm-unknown-linux-gnueabihf
```

### Build Phase

```sh
./build.sh
```

### Distribution

```sh
./dist.sh
```

## Developer Tools

Install the required dependencies: _MuPDF 1.27.2_, _DjVuLibre_, _FreeType_, _HarfBuzz_.

### Emulator

Install one additional dependency: _SDL2_.

You can then run the emulator with:

```sh
./run-emulator.sh
```

### Importer

You can install the importer with:

```sh
./install-importer.sh
```
