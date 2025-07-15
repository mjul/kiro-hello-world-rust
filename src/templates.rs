use askama::Template;

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub error: Option<String>,
}

impl LoginTemplate {
    pub fn new(error: Option<String>) -> Self {
        Self { error }
    }
}

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub username: String,
    pub email: Option<String>,
    pub provider: String,
}

impl DashboardTemplate {
    pub fn new(username: String, email: Option<String>, provider: String) -> Self {
        Self { username, email, provider }
    }
}