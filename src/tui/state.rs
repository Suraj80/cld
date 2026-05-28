#[derive(Default)]
pub struct AppState {
    pub contacts: Vec<String>,
    pub messages: Vec<String>,
    pub input: String,
    pub selected_contact: usize,
}
