//! Example: compress and decompress data using the runtime-loaded lsb-zlib wrapper.
fn main() {
    let z = lsb_zlib::Zlib::load().expect("failed to load zlib");
    println!("zlib version: {}", z.version());

    // Compress
    let input = b"Hello, LSB dynamic loader!";
    let compressed = z.compress_vec(input).expect("compress failed");
    println!(
        "compressed {} bytes -> {} bytes",
        input.len(),
        compressed.len()
    );

    // Decompress
    let decompressed = z
        .uncompress_vec(&compressed, input.len() + 64)
        .expect("uncompress failed");
    assert_eq!(input, &decompressed[..]);
    println!(
        "decompressed {} bytes -> {} bytes",
        compressed.len(),
        decompressed.len()
    );
    println!("round-trip OK: {}", String::from_utf8_lossy(&decompressed));
}
