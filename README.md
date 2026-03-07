# phonikud-rs

Add diacritics to Hebrew text along with phonetic marks.

Rust port of the [phonikud](https://github.com/thewh1teagle/phonikud) project 🤗

## Features

- Phonetics: adds phonetics diacritics
- Fast: 0.1s per sentence (macOS M1) 🚀
- Memory safe: Built with Rust for reliability and performance 🦀
- User friendly: Add diacritics with just a few lines of code ✨
- Lightweight: Runs with ONNX without heavy dependencies 🛠️
- Dual mode: Output nikud male (fully marked) and nikud haser 💡

## Note

This crate uses `ndarray 0.17` which is incompatible with `ndarray 0.16`. If your project depends on `ndarray 0.16`, you will need to upgrade to `0.17`.

## Setup

Add to your `Cargo.toml`:

```toml
[dependencies]
phonikud-rs = "0.1.0"
```

Download required model files:

```console
wget https://huggingface.co/thewh1teagle/phonikud-onnx/resolve/main/phonikud-1.0.int8.onnx -O phonikud.onnx
wget https://huggingface.co/dicta-il/dictabert-large-char-menaked/raw/main/tokenizer.json -O tokenizer.json
```

## Usage

```rust
use phonikud_rs::Phonikud;

fn main() -> anyhow::Result<()> {
    let mut phonikud = Phonikud::new("./phonikud.onnx", "./tokenizer.json")?;
    let text = "שלום עולם";
    let vocalized = phonikud.add_diacritics(text)?;
    println!("{}", vocalized); // שָׁלוֹם עוֹלָם
    Ok(())
}
```

## Examples

See [examples](examples)
