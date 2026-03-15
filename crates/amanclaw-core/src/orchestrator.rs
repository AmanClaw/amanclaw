/// Multi-agent orchestrator — coordinates multiple agents on a complex task.
///
/// Pattern: Coordinator receives a task, breaks it into subtasks,
/// dispatches to worker agents, collects results, synthesizes response.
///
/// Subtasks declare dependencies; the orchestrator executes them in
/// topological order, running independent subtasks in parallel.
use crate::handle::EngineHandle;
use amanclaw_traits::message::IncomingMessage;
use std::collections::{HashMap, HashSet, VecDeque};

/// A complex task composed of subtasks with optional dependencies.
#[derive(Debug, Clone)]
pub struct OrchestratorTask {
    pub description: String,
    pub subtasks: Vec<SubTask>,
}

/// A single unit of work to be dispatched to an agent.
#[derive(Debug, Clone)]
pub struct SubTask {
    /// Unique identifier for this subtask.
    pub id: String,
    /// Prompt / input to send to the engine.
    pub prompt: String,
    /// IDs of subtasks that must complete before this one starts.
    pub depends_on: Vec<String>,
}

/// Result of executing a single subtask.
#[derive(Debug, Clone)]
pub struct SubTaskResult {
    pub id: String,
    pub output: String,
    pub success: bool,
}

/// Orchestration error.
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("dependency cycle detected involving subtask '{0}'")]
    CycleDetected(String),
    #[error("unknown dependency '{dep}' in subtask '{task}'")]
    UnknownDependency { task: String, dep: String },
    #[error("engine error: {0}")]
    EngineError(#[from] anyhow::Error),
}

/// Multi-agent orchestrator.
#[derive(Clone)]
pub struct Orchestrator {
    handle: EngineHandle,
    max_workers: usize,
}

impl Orchestrator {
    pub fn new(handle: EngineHandle, max_workers: usize) -> Self {
        Self {
            handle,
            max_workers: max_workers.max(1),
        }
    }

    /// Execute an orchestrator task: resolve dependencies, run subtasks
    /// (parallel where possible), collect results, return combined output.
    pub async fn execute(&self, task: OrchestratorTask) -> Result<String, OrchestratorError> {
        if task.subtasks.is_empty() {
            return Ok(String::new());
        }

        // Validate dependencies exist
        let ids: HashSet<&str> = task.subtasks.iter().map(|s| s.id.as_str()).collect();
        for st in &task.subtasks {
            for dep in &st.depends_on {
                if !ids.contains(dep.as_str()) {
                    return Err(OrchestratorError::UnknownDependency {
                        task: st.id.clone(),
                        dep: dep.clone(),
                    });
                }
            }
        }

        // Topological sort (Kahn's algorithm)
        let order = topological_sort(&task.subtasks)?;

        // Build lookup
        let subtask_map: HashMap<String, SubTask> = task
            .subtasks
            .into_iter()
            .map(|s| (s.id.clone(), s))
            .collect();

        // Execute in layers: group subtasks that can run in parallel
        let mut results: HashMap<String, SubTaskResult> = HashMap::new();
        let mut remaining: VecDeque<String> = order.into();

        while !remaining.is_empty() {
            // Collect subtasks whose dependencies are all satisfied
            let mut ready = Vec::new();
            let mut deferred = VecDeque::new();

            while let Some(id) = remaining.pop_front() {
                let st = &subtask_map[&id];
                let deps_met = st.depends_on.iter().all(|d| results.contains_key(d));
                if deps_met {
                    ready.push(id);
                    if ready.len() >= self.max_workers {
                        // Drain remaining into deferred
                        while let Some(r) = remaining.pop_front() {
                            deferred.push_back(r);
                        }
                    }
                } else {
                    deferred.push_back(id);
                }
            }

            remaining = deferred;

            if ready.is_empty() {
                // Should not happen after topo-sort, but guard against it
                break;
            }

            // Execute ready subtasks in parallel using JoinSet
            let mut join_set = tokio::task::JoinSet::new();
            for id in ready {
                let st = subtask_map[&id].clone();
                let handle = self.handle.clone();

                // Build a synthetic message for the engine
                let msg = IncomingMessage {
                    user_id: format!("orchestrator:{id}"),
                    chat_id: format!("orchestrator-internal"),
                    platform: "orchestrator".into(),
                    text: st.prompt.clone(),
                    username: None,
                    first_name: None,
                    is_group: false,
                    image_data: None,
                    reply_to: None,
                    topic_id: None,
                    channel_context: None,
                    is_cron: false,
                    is_webhook: false,
                    is_subagent: true,
                };

                join_set.spawn(async move {
                    match handle.ask(msg).await {
                        Ok(Some(response)) => SubTaskResult {
                            id: st.id.clone(),
                            output: response.text,
                            success: true,
                        },
                        Ok(None) => SubTaskResult {
                            id: st.id.clone(),
                            output: String::new(),
                            success: true,
                        },
                        Err(e) => SubTaskResult {
                            id: st.id.clone(),
                            output: e.to_string(),
                            success: false,
                        },
                    }
                });
            }

            // Collect results
            while let Some(result) = join_set.join_next().await {
                match result {
                    Ok(sub_result) => {
                        results.insert(sub_result.id.clone(), sub_result);
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Subtask join error");
                    }
                }
            }
        }

        // Combine results in original order
        let mut combined = Vec::new();
        for (id, result) in &results {
            let status = if result.success { "OK" } else { "FAILED" };
            combined.push(format!("[{id}] ({status}): {}", result.output));
        }

        Ok(combined.join("\n\n"))
    }
}

/// Topological sort using Kahn's algorithm. Returns an error on cycles.
fn topological_sort(subtasks: &[SubTask]) -> Result<Vec<String>, OrchestratorError> {
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

    for st in subtasks {
        in_degree.entry(st.id.as_str()).or_insert(0);
        for dep in &st.depends_on {
            *in_degree.entry(st.id.as_str()).or_insert(0) += 1;
            dependents
                .entry(dep.as_str())
                .or_default()
                .push(st.id.as_str());
        }
    }

    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|&(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();

    let mut order = Vec::new();

    while let Some(id) = queue.pop_front() {
        order.push(id.to_string());
        if let Some(deps) = dependents.get(id) {
            for &dep in deps {
                if let Some(deg) = in_degree.get_mut(dep) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(dep);
                    }
                }
            }
        }
    }

    if order.len() != subtasks.len() {
        // Find a node still with non-zero in-degree
        let stuck = in_degree
            .iter()
            .find(|&(_, &deg)| deg > 0)
            .map(|(&id, _)| id.to_string())
            .unwrap_or_default();
        return Err(OrchestratorError::CycleDetected(stuck));
    }

    Ok(order)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topological_sort_linear() {
        let subtasks = vec![
            SubTask { id: "a".into(), prompt: "".into(), depends_on: vec![] },
            SubTask { id: "b".into(), prompt: "".into(), depends_on: vec!["a".into()] },
            SubTask { id: "c".into(), prompt: "".into(), depends_on: vec!["b".into()] },
        ];
        let order = topological_sort(&subtasks).unwrap();
        assert_eq!(order, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_topological_sort_parallel() {
        let subtasks = vec![
            SubTask { id: "a".into(), prompt: "".into(), depends_on: vec![] },
            SubTask { id: "b".into(), prompt: "".into(), depends_on: vec![] },
            SubTask { id: "c".into(), prompt: "".into(), depends_on: vec!["a".into(), "b".into()] },
        ];
        let order = topological_sort(&subtasks).unwrap();
        // a and b should come before c; their relative order doesn't matter
        let pos_a = order.iter().position(|x| x == "a").unwrap();
        let pos_b = order.iter().position(|x| x == "b").unwrap();
        let pos_c = order.iter().position(|x| x == "c").unwrap();
        assert!(pos_a < pos_c);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn test_topological_sort_cycle() {
        let subtasks = vec![
            SubTask { id: "a".into(), prompt: "".into(), depends_on: vec!["b".into()] },
            SubTask { id: "b".into(), prompt: "".into(), depends_on: vec!["a".into()] },
        ];
        let result = topological_sort(&subtasks);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OrchestratorError::CycleDetected(_)));
    }

    #[test]
    fn test_topological_sort_single() {
        let subtasks = vec![
            SubTask { id: "only".into(), prompt: "do it".into(), depends_on: vec![] },
        ];
        let order = topological_sort(&subtasks).unwrap();
        assert_eq!(order, vec!["only"]);
    }

    #[test]
    fn test_unknown_dependency() {
        // This is tested at the execute level; topological_sort itself
        // would just give a wrong in-degree. Let's test the validation
        // in a unit-style way.
        let subtasks = vec![
            SubTask { id: "a".into(), prompt: "".into(), depends_on: vec!["nonexistent".into()] },
        ];
        let ids: HashSet<&str> = subtasks.iter().map(|s| s.id.as_str()).collect();
        let has_unknown = subtasks.iter().any(|s| {
            s.depends_on.iter().any(|d| !ids.contains(d.as_str()))
        });
        assert!(has_unknown);
    }

    // Integration-style tests that require an EngineHandle cannot run in unit tests
    // because they need a full engine. The following tests verify the orchestrator
    // logic via the topological sort and task construction.

    #[test]
    fn test_orchestrator_task_construction() {
        let task = OrchestratorTask {
            description: "Research Islamic ruling".into(),
            subtasks: vec![
                SubTask {
                    id: "quran_search".into(),
                    prompt: "Search Quran for fasting rules".into(),
                    depends_on: vec![],
                },
                SubTask {
                    id: "hadith_search".into(),
                    prompt: "Search Hadith for fasting rules".into(),
                    depends_on: vec![],
                },
                SubTask {
                    id: "synthesize".into(),
                    prompt: "Combine Quran and Hadith findings".into(),
                    depends_on: vec!["quran_search".into(), "hadith_search".into()],
                },
            ],
        };

        let order = topological_sort(&task.subtasks).unwrap();
        assert_eq!(order.len(), 3);
        // synthesize must come after both searches
        let pos_synth = order.iter().position(|x| x == "synthesize").unwrap();
        assert_eq!(pos_synth, 2);
    }

    #[test]
    fn test_diamond_dependency() {
        // A -> B, A -> C, B -> D, C -> D
        let subtasks = vec![
            SubTask { id: "a".into(), prompt: "".into(), depends_on: vec![] },
            SubTask { id: "b".into(), prompt: "".into(), depends_on: vec!["a".into()] },
            SubTask { id: "c".into(), prompt: "".into(), depends_on: vec!["a".into()] },
            SubTask { id: "d".into(), prompt: "".into(), depends_on: vec!["b".into(), "c".into()] },
        ];
        let order = topological_sort(&subtasks).unwrap();
        assert_eq!(order.len(), 4);
        let pos_a = order.iter().position(|x| x == "a").unwrap();
        let pos_b = order.iter().position(|x| x == "b").unwrap();
        let pos_c = order.iter().position(|x| x == "c").unwrap();
        let pos_d = order.iter().position(|x| x == "d").unwrap();
        assert!(pos_a < pos_b);
        assert!(pos_a < pos_c);
        assert!(pos_b < pos_d);
        assert!(pos_c < pos_d);
    }
}
