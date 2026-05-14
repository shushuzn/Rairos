use pyo3::prelude::*;
use rairos_core::Paper;
use rairos_research::{DeepResearchAgent, DeepResearchConfig, GapSnapshot, PaperSnapshot, ResearchBackend, AgentThought};

fn call_json_fn(_py: Python<'_>, func: &Bound<'_, PyAny>, args_json: &str) -> PyResult<String> {
    let result = func.call1((args_json,))?;
    result.extract::<String>()
}

fn call_void_fn(_py: Python<'_>, func: &Bound<'_, PyAny>, args_json: &str) -> PyResult<()> {
    func.call1((args_json,))?;
    Ok(())
}

// ─── Python callback backend ──────────────────────────────────────────────────

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
    fn analyze_gaps(&self, topic: &str, snapshots: &[PaperSnapshot]) -> Result<Vec<GapSnapshot>, String> {
        Python::with_gil(|py| {
            let args = serde_json::json!({"topic": topic, "snapshots": snapshots}).to_string();
            let json_str = call_json_fn(py, self.analyze_gaps.bind(py), &args.to_string())
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
    fn encode_accepted_gap(&self, topic: &str, gap: &GapSnapshot) -> Result<(), String> {
        Python::with_gil(|py| {
            let args = serde_json::json!({"topic": topic, "gap": gap}).to_string();
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

// ─── No-op backend (all defaults) ─────────────────────────────────────────────

struct DefaultBackendNoop;

impl ResearchBackend for DefaultBackendNoop {}

// ─── PyResearchAgent ──────────────────────────────────────────────────────────

#[pyclass]
struct PyResearchAgent {
    agent: DeepResearchAgent,
    backend: Option<PyResearchBackend>,
    stop_requested: bool,
    py_db: Option<Py<PyAny>>,
}

#[pymethods]
impl PyResearchAgent {
    #[new]
    #[pyo3(signature = (query, config_json, stream_plan=None, search_papers=None, extract_paper=None, analyze_gaps=None, get_search_guidance=None, encode_accepted_gap=None, on_thought=None, find_skills=None, checkpoint=None, new_session=None))]
    fn new(
        query: &str,
        config_json: &str,
        stream_plan: Option<PyObject>,
        search_papers: Option<PyObject>,
        extract_paper: Option<PyObject>,
        analyze_gaps: Option<PyObject>,
        get_search_guidance: Option<PyObject>,
        encode_accepted_gap: Option<PyObject>,
        on_thought: Option<PyObject>,
        find_skills: Option<PyObject>,
        checkpoint: Option<PyObject>,
        new_session: Option<PyObject>,
    ) -> PyResult<Self> {
        let config: DeepResearchConfig = serde_json::from_str(config_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let agent = DeepResearchAgent::new(query, config);

        Ok(Self {
            agent,
            backend: stream_plan.map(|sp| PyResearchBackend {
                stream_plan: sp,
                search_papers: search_papers.unwrap(),
                extract_paper: extract_paper.unwrap(),
                analyze_gaps: analyze_gaps.unwrap(),
                get_search_guidance: get_search_guidance.unwrap(),
                encode_accepted_gap: encode_accepted_gap.unwrap(),
                on_thought: on_thought.unwrap(),
                find_skills: find_skills.unwrap(),
                checkpoint: checkpoint.unwrap(),
                new_session: new_session.unwrap(),
            }),
            stop_requested: false,
            py_db: None,
        })
    }

    #[pyo3(signature = (mode="agent"))]
    fn run(&mut self, mode: &str) -> PyResult<String> {
        let noop = DefaultBackendNoop;
        let backend: &dyn ResearchBackend = self.backend.as_ref().map(|b| b as &dyn ResearchBackend).unwrap_or(&noop);
        let result = self.agent.run(backend, mode, self.stop_requested);
        serde_json::to_string(&result)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    fn stop(&mut self) {
        self.stop_requested = true;
    }

    #[getter(_stop_requested)]
    fn get_stop_requested(&self) -> bool { self.stop_requested }
    #[setter(_stop_requested)]
    fn set_stop_requested(&mut self, val: bool) { self.stop_requested = val; }

    #[getter] fn query(&self) -> &str { &self.agent.query }
    #[getter] fn verbose(&self) -> bool { self.agent.deep_config().verbose }
    #[getter] fn max_iterations(&self) -> i32 { self.agent.deep_config().max_iterations }
    #[getter] fn max_papers_per_iteration(&self) -> usize { self.agent.deep_config().max_papers_per_iteration }

    #[getter]
    fn session_id(&self) -> String {
        self.agent.session_id().to_string()
    }

    #[getter]
    fn get_db(&self, py: Python<'_>) -> Option<PyObject> {
        self.py_db.as_ref().map(|d| d.clone_ref(py))
    }

    #[setter]
    fn set_db(&mut self, val: Option<PyObject>) {
        self.py_db = val;
    }

    fn start(&self) -> String {
        use rairos_research::snapstate::Snapstate;
        let store = Snapstate::new(None);
        let session = store.new_session(
            &self.agent.query,
            self.agent.deep_config().max_iterations,
        );
        let _ = store.save(&session);
        session.session_id
    }
}

#[pymodule]
fn rairos_research_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyResearchAgent>()?;
    Ok(())
}
