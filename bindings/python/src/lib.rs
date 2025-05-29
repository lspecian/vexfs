use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use ::vexfs::vexfs_api::VexFS;

// Global VexFS instance wrapped in Arc<Mutex<>> for thread safety
static mut VEXFS_INSTANCE: Option<Arc<Mutex<VexFS>>> = None;

/// VexFS Python Client
#[pyclass]
struct VexFSClient {
    vexfs: Arc<Mutex<VexFS>>,
}

#[pymethods]
impl VexFSClient {
    #[new]
    fn new() -> Self {
        Self {
            vexfs: Arc::new(Mutex::new(VexFS::new())),
        }
    }

    /// Initialize VexFS with a mount point
    fn init(&self, mount_point: String) -> PyResult<()> {
        let vexfs = VexFS::init(&mount_point)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to initialize VexFS: {}", e)
            ))?;
        
        *self.vexfs.lock().unwrap() = vexfs;
        Ok(())
    }

    /// Add a document with automatic embedding generation
    fn add(&self, text: String, metadata: Option<&PyDict>) -> PyResult<String> {
        let metadata_map = if let Some(meta) = metadata {
            let mut map = HashMap::new();
            for (key, value) in meta.iter() {
                let key_str = key.extract::<String>()?;
                let value_json = python_to_json_value(value)?;
                map.insert(key_str, value_json);
            }
            Some(map)
        } else {
            None
        };

        let mut vexfs = self.vexfs.lock().unwrap();
        let doc_id = vexfs.add(&text, metadata_map)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to add document: {}", e)
            ))?;
        
        Ok(doc_id)
    }

    /// Query for similar documents using a vector
    fn query(&self, vector: Vec<f32>, top_k: Option<usize>) -> PyResult<Vec<(String, f32, Option<String>)>> {
        let k = top_k.unwrap_or(10);
        let vexfs = self.vexfs.lock().unwrap();
        
        let results = vexfs.query(vector, k)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to query documents: {}", e)
            ))?;
        
        Ok(results)
    }

    /// Delete a document by ID
    fn delete(&self, doc_id: String) -> PyResult<()> {
        let mut vexfs = self.vexfs.lock().unwrap();
        vexfs.delete(&doc_id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to delete document: {}", e)
            ))?;
        
        Ok(())
    }

    /// Get VexFS statistics
    fn stats(&self) -> PyResult<HashMap<String, String>> {
        let vexfs = self.vexfs.lock().unwrap();
        let stats = vexfs.stats()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to get stats: {}", e)
            ))?;
        
        // Convert serde_json::Value to String for Python compatibility
        let mut result = HashMap::new();
        for (key, value) in stats {
            result.insert(key, value.to_string());
        }
        
        Ok(result)
    }

    /// Get version information
    fn version(&self) -> PyResult<HashMap<String, String>> {
        let vexfs = self.vexfs.lock().unwrap();
        Ok(vexfs.version())
    }
}

// Helper function to convert Python values to serde_json::Value
fn python_to_json_value(value: &PyAny) -> PyResult<serde_json::Value> {
    if let Ok(s) = value.extract::<String>() {
        Ok(serde_json::Value::String(s))
    } else if let Ok(i) = value.extract::<i64>() {
        Ok(serde_json::Value::Number(serde_json::Number::from(i)))
    } else if let Ok(f) = value.extract::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(f) {
            Ok(serde_json::Value::Number(num))
        } else {
            Ok(serde_json::Value::Null)
        }
    } else if let Ok(b) = value.extract::<bool>() {
        Ok(serde_json::Value::Bool(b))
    } else {
        Ok(serde_json::Value::String(value.to_string()))
    }
}

// Legacy functions for backward compatibility
#[pyfunction]
fn init(mount_point: String) -> PyResult<()> {
    let vexfs = VexFS::init(&mount_point)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to initialize VexFS: {}", e)
        ))?;
    
    unsafe {
        VEXFS_INSTANCE = Some(Arc::new(Mutex::new(vexfs)));
    }
    
    Ok(())
}

#[pyfunction]
fn add(text: String, metadata: Option<HashMap<String, String>>) -> PyResult<String> {
    let vexfs_instance = unsafe {
        VEXFS_INSTANCE.as_ref().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "VexFS not initialized. Call vexfs.init() first."
            )
        })?
    };
    
    let metadata_map = metadata.map(|m| {
        m.into_iter()
            .map(|(k, v)| (k, serde_json::Value::String(v)))
            .collect()
    });

    let mut vexfs = vexfs_instance.lock().unwrap();
    let doc_id = vexfs.add(&text, metadata_map)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to add document: {}", e)
        ))?;
    
    Ok(doc_id)
}

#[pyfunction]
fn query(vector: Vec<f32>, top_k: usize) -> PyResult<Vec<(String, f32, Option<String>)>> {
    let vexfs_instance = unsafe {
        VEXFS_INSTANCE.as_ref().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "VexFS not initialized. Call vexfs.init() first."
            )
        })?
    };
    
    let vexfs = vexfs_instance.lock().unwrap();
    let results = vexfs.query(vector, top_k)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to query documents: {}", e)
        ))?;
    
    Ok(results)
}

#[pyfunction]
fn delete(doc_id: String) -> PyResult<()> {
    let vexfs_instance = unsafe {
        VEXFS_INSTANCE.as_ref().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "VexFS not initialized. Call vexfs.init() first."
            )
        })?
    };
    
    let mut vexfs = vexfs_instance.lock().unwrap();
    vexfs.delete(&doc_id)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to delete document: {}", e)
        ))?;
    
    Ok(())
}

#[pymodule]
fn vexfs(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<VexFSClient>()?;
    m.add_function(wrap_pyfunction!(init, m)?)?;
    m.add_function(wrap_pyfunction!(add, m)?)?;
    m.add_function(wrap_pyfunction!(query, m)?)?;
    m.add_function(wrap_pyfunction!(delete, m)?)?;
    Ok(())
}