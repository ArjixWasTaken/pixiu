fn main() {
    // Compiles styles.css (the theme tokens plus every Tailwind class used in
    // src/) into a stylesheet asset. The Tailwind CLI is downloaded, unless
    // PIXIU_TAILWIND names one: the downloaded build needs glibc, so the
    // Alpine (musl) image build brings its own.
    let mut config = topcoat::tailwind::BuildConfig::new().input("styles.css");
    if let Some(executable) = std::env::var_os("PIXIU_TAILWIND") {
        config = config.executable(executable);
    }
    config.render().unwrap();
}
