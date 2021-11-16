use windres::Build;

fn main() {
    Build::new().compile("runcat.rc").unwrap();
}
