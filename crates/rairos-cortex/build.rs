fn main() {
    // Workaround for gcc 10/11 bug when compiling aws-lc-sys assembly
    // See: https://github.com/aws/aws-lc-rs/issues/672
    println!("cargo:rerun-if-env-changed=AWS_LC_SYS_NOASM");
    if std::env::var("AWS_LC_SYS_NOASM").is_err() {
        std::env::set_var("AWS_LC_SYS_NOASM", "1");
    }
}
