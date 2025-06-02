fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile(
            &[
                "../proto/wallet.proto",
                "../proto/transaction.proto", 
                "../proto/defi.proto",
                "../proto/solana.proto",
                "../proto/health.proto",
            ],
            &["../proto"],
        )?;
    Ok(())
}
