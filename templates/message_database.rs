pub struct Message {
    created_at: chrono::DateTime<chrono::Utc>,
    user: String,
    text: String,
}

pub type Messages = Vec<Message>;
