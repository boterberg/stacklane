pub mod pterodactyl;

#[derive(Default)]
pub struct Report {
    pub users_created: usize,
    pub users_matched: usize,
    pub services_created: usize,
    pub services_existing: usize,
    pub skipped: Vec<String>,
}
