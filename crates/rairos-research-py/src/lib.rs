use pyo3::prelude::*;
use rairos_core::Paper;
use rairos_research::{DeepResearchAgent, DeepResearchConfig, GapSnapshot, PaperSnapshot, ResearchBackend, AgentThought};

/// Calls a Python callback with given args, returns JSON string result.
fn call_json_fn(py: Python<'_>, func: &Bound<'_, PyAny>, args_json: &str) -> PyResult<String> {
    let result = func.call1((args_json,))?;
    result.extract::<String>()
}

fn call_void_fn(py: Python<'_>, func: &Bound<'_, PyAny>, args_json: &str) -> PyResult<()> {
    func.call1((args_json,))?;
    Ok(())
}

/// PyResearchBackend — calls Python functions for each backend operation.
struct PyResearchBackend {
    stream_plan: PyObject,
    search_papers: PyObject,
    extract_paper: PyObject,
    analyze_gaps: PyObject,
    get_search_guidance: PyObject,
    encode_accepted_gap: PyObject,
    on_thought: PyObject,
    find_skills: PyObject,
    checkpoint: PyObject,
    new_session: PyObject,
}

impl ResearchBackend for PyResearchBackend {
    fn stream_plan(&self, query: &str, iteration: i32) -> Result<String, String> {
        Python::with_gil(|py| {
            let args = serde_json::json!({"query": query, "iteration": iteration});
            call_json_fn(py, self.stream_plan.bind(py), &args.to_string())
                .map_err(|e| e.to_string())
        })
    }

    fn search_papers(&self, query: &str, max: usize) -> Result<Vec<Paper>, String> {
        Python::with_gil(|py| {
            let args = serde_json::json!({"query": query, "max": max});
            let json_str = call_json_fn(py, self.search_papers.bind(py), &args.to_string())
                .map_err(|e| e.to_string())?;
            serde_json::from_str(&json_str).map_err(|e| e.to_string())
        })
    }

    fn extract_paper(&self, paper: &Paper) -> Result<PaperSnapshot, String> {
        Python::with_gil(|py| {
            let args = serde_json::to_string(paper).unwrap_or_default();
            let json_str = call_json_fn(py, self.extract_paper.bind(py), &args)
                .map_err(|e| e.to_string())?;
            serde_json::from_str(&json_str).map_err(|e| e.to_string())
        })
    }

    fn analyze_gaps(&self, snapshots: &[PaperSnapshot]) -> Result<Vec<GapSnapshot>, String> {
        Python::with_gil(|py| {
            let args = serde_json::to_string(snapshots).unwrap_or_default();
            let json_str = call_json_fn(py, self.analyze_gaps.bind(py), &args)
                .map_err(|e| e.to_string())?;
            serde_json::from_str(&json_str).map_err(|e| e.to_string())
        })
    }

    fn get_search_guidance(&self, topic: &str, gap_type: &str, gap_title: &str) -> Result<(Option<String>, f64), String> {
        Python::with_gil(|py| {
            let args = serde_json::json!({"topic": topic, "gap_type": gap_type, "gap_title": gap_title});
            let json_str = call_json_fn(py, self.get_search_guidance.bind(py), &args.to_string())
                .map_err(|e| e.to_string())?;
            let result: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;
            let hint = result["hint"].as_str().map(|s| s.to_string());
            let confidence = result["confidence"].as_f64().unwrap_or(0.0);
            Ok((hint, confidence))
        })
    }

    fn encode_accepted_gap(&self, gap: &GapSnapshot) -> Result<(), String> {
        Python::with_gil(|py| {
            let args = serde_json::to_string(gap).unwrap_or_default();
            call_void_fn(py, self.encode_accepted_gap.bind(py), &args)
                .map_err(|e| e.to_string())
        })
    }

    fn on_thought(&self, thought: &AgentThought) -> Result<(), String> {
        Python::with_gil(|py| {
            let args = serde_json::to_string(thought).unwrap_or_default();
            call_void_fn(py, self.on_thought.bind(py), &args)
                .map_err(|e| e.to_string())
        })
    }

    fn find_skills(&self, query: &str) -> Result<Vec<String>, String> {
        Python::with_gil(|py| {
            let json_str = call_json_fn(py, self.find_skills.bind(py), query)
                .map_err(|e| e.to_string())?;
            serde_json::from_str(&json_str).map_err(|e| e.to_string())
        })
    }

    fn checkpoint(&self, session_json: &str) -> Result<(), String> {
        Python::with_gil(|py| {
            call_void_fn(py, self.checkpoint.bind(py), session_json)
                .map_err(|e| e.to_string())
        })
    }

    fn new_session(&self, query: &str, max_iterations: i32) -> Result<String, String> {
        Python::with_gil(|py| {
            let args = serde_json::json!({"query": query, "max_iterations": max_iterations});
            call_json_fn(py, self.new_session.bind(py), &args.to_string())
                .map_err(|e| e.to_string())
        })
    }
}

#[pyclass]
struct PyResearchAgent {
    agent: DeepResearchAgent,
    backend: PyResearchBackend,
}

#[pymethods]
impl PyResearchAgent {
    #[new]
    #[pyo3(signature = (query, config_json, stream_plan, search_papers, extract_paper, analyze_gaps, get_search_guidance, encode_accepted_gap, on_thought, find_skills, checkpoint, new_session))]
    fn new(
        query: &str,
        config_json: &str,
        stream_plan: PyObject,
        search_papers: PyObject,
        extract_paper: PyObject,
        analyze_gaps: PyObject,
        get_search_guidance: PyObject,
        encode_accepted_gap: PyObject,
        on_thought: PyObject,
        find_skills: PyObject,
        checkpoint: PyObject,
        new_session: PyObject,
    ) -> PyResult<Self> {
        let config: DeepResearchConfig = serde_json::from_str(config_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(Self {
            agent: DeepResearchAgent::new(query, config),
            backend: PyResearchBackend {
                stream_plan,
                search_papers,
                extract_paper,
                analyze_gaps,
                get_search_guidance,
                encode_accepted_gap,
                on_thought,
                find_skills,
                checkpoint,
                new_session,
            },
        })
    }

    #[pyo3(signature = (mode="agent", stop_requested=false))]
    fn run(&mut self, mode: &str, stop_requested: bool) -> PyResult<String> {
        let result = self.agent.run(&self.backend, mode, stop_requested);
        serde_json::to_string(&result)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
}

#[pymodule]
fn rairos_research_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyResearchAgent>()?;
    Ok(())
}
