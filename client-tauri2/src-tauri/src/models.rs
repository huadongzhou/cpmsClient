use serde::Serialize;

#[derive(Serialize)]
pub struct CommandResult<T>
where
    T: Serialize,
{
    pub success: bool,
    pub code: String,
    pub message: String,
    pub data: Option<T>,
    pub logs: Vec<String>,
}

impl<T> CommandResult<T>
where
    T: Serialize,
{
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            code: "OK".into(),
            message: "success".into(),
            data: Some(data),
            logs: Vec::new(),
        }
    }

    pub fn fail(code: &str, message: &str) -> Self {
        Self {
            success: false,
            code: code.into(),
            message: message.into(),
            data: None,
            logs: Vec::new(),
        }
    }
}
