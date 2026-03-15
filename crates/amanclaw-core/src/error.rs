/// Typed pipeline errors for middleware to return and callers to match on.
#[derive(thiserror::Error, Debug)]
pub enum PipelineError {
    #[error("user blocked")]
    UserBlocked,

    #[error("user pending approval")]
    UserPending,

    #[error("rate limited")]
    RateLimited,

    #[error("injection detected")]
    InjectionDetected,

    #[error("llm error: {0}")]
    LlmError(String),

    #[error("skill error: {skill}: {message}")]
    SkillError { skill: String, message: String },

    #[error("context budget exceeded: needed {needed} tokens, only {available} available")]
    ContextBudgetExceeded { needed: usize, available: usize },

    #[error("engine shutting down")]
    EngineShutdown,

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
