fn main() {
    // Compiles styles.css (the theme tokens plus every Tailwind class used in
    // src/) into a stylesheet asset.
    topcoat::tailwind::BuildConfig::new()
        .input("styles.css")
        .render()
        .unwrap();
}
