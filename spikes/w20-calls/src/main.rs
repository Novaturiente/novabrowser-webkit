//! W20 spike (A1): list every WebKit runtime feature, tab-separated:
//! identifier, category, status, on by default, on in a fresh Settings.
//! `cargo run > features-<version>.tsv`
use webkit6::Settings;

fn main() {
    gtk::init().expect("gtk init");
    use webkit6::functions::{major_version, micro_version, minor_version};
    println!("# WebKitGTK {}.{}.{}", major_version(), minor_version(), micro_version());
    println!("identifier\tcategory\tstatus\tdefault_on\tfresh_on");
    let s = Settings::new();
    let all = Settings::all_features().expect("feature list");
    for f in (0..all.length()).filter_map(|i| all.get(i)) {
        println!(
            "{}\t{}\t{:?}\t{}\t{}",
            f.identifier().unwrap_or_default(),
            f.category().unwrap_or_default(),
            f.status(),
            f.is_default_value(),
            s.is_feature_enabled(&f)
        );
    }
}
