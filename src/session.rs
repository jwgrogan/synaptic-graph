// Session state management

#[derive(Debug)]
pub struct Session {
    id: String,
    incognito: bool,
}

impl Session {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            incognito: false,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn is_incognito(&self) -> bool {
        self.incognito
    }

    pub fn set_incognito(&mut self, incognito: bool) {
        self.incognito = incognito;
    }
}
