use std::str::FromStr;

#[path = "src/version/mod.rs"]
mod version;

use version::Version;

fn main() {
    let test_cases = vec\![
        "17.0.2+8-LTS",
        "11.0.21+9-LTS-3299655",
    ];

    for test in test_cases {
        println\!("\nParsing: {}", test);
        match Version::from_str(test) {
            Ok(v) => {
                println\!("  components: {:?}", v.components);
                println\!("  build: {:?}", v.build);
                println\!("  pre_release: {:?}", v.pre_release);
            }
            Err(e) => println\!("  Error: {:?}", e),
        }
    }
}
EOF < /dev/null
