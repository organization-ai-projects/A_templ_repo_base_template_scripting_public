pub(crate) struct StableDepsPlaceholder;

impl StableDepsPlaceholder {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn run(&self) {
        println!(
            "Stable dependency verification is not yet defined for the multi-repository template."
        );
        println!(
            "Replace this placeholder workflow when repository dependency boundaries are formalized."
        );
    }
}
