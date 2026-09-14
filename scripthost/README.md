# script-host

QuickJS kompilerad till wasm. Byggs separat från huvud-workspacet eftersom
den har en annan target.

```bash
rustup target add wasm32-wasip1
apt-get install clang lld wasi-libc

# Debians wasi-libc ligger utspritt; clang vill ha en sysroot-struktur.
mkdir -p /tmp/wasi-sysroot/include /tmp/wasi-sysroot/lib/wasm32-wasi
cp -r /usr/include/wasm32-wasi/* /tmp/wasi-sysroot/include/
cp -r /usr/lib/wasm32-wasi/*     /tmp/wasi-sysroot/lib/wasm32-wasi/

CC_wasm32_wasip1=clang \
CFLAGS_wasm32_wasip1="--target=wasm32-wasi --sysroot=/tmp/wasi-sysroot" \
  cargo build --release --target wasm32-wasip1

cp target/wasm32-wasip1/release/script_host.wasm ../assets/
```

Resultatet är ~782 KB och innehåller hela JS-motorn. Den laddas en gång och
kör alla skript; modulen behöver alltså inte byggas om när du skriver skript.
