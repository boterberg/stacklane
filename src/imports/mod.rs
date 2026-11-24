pub mod pterodactyl;
pub mod text;
pub mod whmcs;

pub struct Count {
    pub label: &'static str,
    pub created: usize,
    pub existing: usize,
}

#[derive(Default)]
pub struct Report {
    pub counts: Vec<Count>,
    pub skipped: Vec<String>,
    pub notes: Vec<String>,
}

impl Report {
    pub fn count(&mut self, label: &'static str) -> &mut Count {
        if let Some(index) = self.counts.iter().position(|count| count.label == label) {
            return &mut self.counts[index];
        }
        self.counts.push(Count {
            label,
            created: 0,
            existing: 0,
        });
        self.counts.last_mut().expect("just pushed")
    }

    pub fn created(&mut self, label: &'static str) {
        self.count(label).created += 1;
    }

    pub fn existing(&mut self, label: &'static str) {
        self.count(label).existing += 1;
    }
}
