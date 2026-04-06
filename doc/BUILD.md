# Build

Start by cloning the repository:

```sh
git clone https://github.com/baskerville/plato.git
cd plato
```

## Plato

#### Preliminary

Install the required dependencies: `wget`, `curl`, `git`, `pkg-config`, `unzip`, `jq`, `patchelf`.

**ARM cross-compiler (macOS):**

Install the toolchain via the [messense tap](https://github.com/messense/homebrew-macos-cross-toolchains):

```sh
brew tap messense/macos-cross-toolchains
brew install arm-unknown-linux-gnueabihf
```

The compiler installs to a Cellar path that is not automatically added to `$PATH` (the separate `arm-linux-gnueabihf-binutils` package occupies those symlinks). Add the compiler bin directory to your path:

```sh
export PATH="/opt/homebrew/Cellar/arm-unknown-linux-gnueabihf/15.2.0/bin:$PATH"
```

Add this to your shell profile (`~/.zshrc` or `~/.bashrc`) to make it permanent.

**Collatinus (Latin morphological analysis):**

Plato includes support for Latin dictionaries via [Collatinus](https://github.com/biblissima/collatinus). Building the Collatinus wrapper requires Qt 5:

```sh
brew install qt@5
```

Download the Collatinus source:

```sh
cd thirdparty && ./download.sh collatinus && cd ..
```

The Collatinus wrapper (`collatinus_wrapper/build-kobo.sh`) is built automatically as part of `./build.sh`.

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

The distribution includes `data/` from Collatinus (Latin lexicon and morphological data files). These are looked up at runtime relative to the Plato binary (`<binary-dir>/data/`).

## Developer Tools

Install the required dependencies: _MuPDF 1.27.2_, _DjVuLibre_, _FreeType_, _HarfBuzz_.

**Collatinus (Latin dictionaries in the emulator):**

Install Qt 5 and download the Collatinus source (see above), then build the macOS wrapper:

```sh
cd collatinus_wrapper && ./build.sh && cd ..
```

Create a symlink so the emulator finds the data files:

```sh
ln -s thirdparty/collatinus/bin/data target/debug/data
```

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
