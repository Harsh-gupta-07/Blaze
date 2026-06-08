pub fn theme_default() -> String {
    return "dark".into();
}

pub fn ignored_dirs_defaults() -> Vec<String> {
    return vec!["node_modules".into(), "__pycache__".into()];
}

pub fn walker_dirs_defaults() -> Vec<String> {
    return vec!["/Users".into(), "/Applications".into()];
}
