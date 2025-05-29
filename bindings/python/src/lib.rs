use pyo3::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

static mut MOUNT_POINT: Option<String> = None;

#[pyfunction]
fn init(mount_point: String) -> PyResult<()> {
    // Verify mount point exists and is accessible
    if !Path::new(&mount_point).exists() {
        return Err(PyErr::new::<pyo3::exceptions::PyFileNotFoundError, _>(
            format!("Mount point {} does not exist", mount_point)
        ));
    }
    
    unsafe {
        MOUNT_POINT = Some(mount_point);
    }
    
    Ok(())
}

#[pyfunction]
fn add(text: String, metadata: HashMap<String, String>) -> PyResult<String> {
    let mount_point = unsafe {
        MOUNT_POINT.as_ref().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "VexFS not initialized. Call vexfs.init() first."
            )
        })?
    };
    
    // Generate unique document ID
    let doc_id = format!("doc_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        rand::random::<u32>()
    );
    
    // Write document to filesystem
    let doc_path = format!("{}/documents/{}.txt", mount_point, doc_id);
    fs::create_dir_all(format!("{}/documents", mount_point))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
    
    fs::write(&doc_path, &text)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
    
    // Write metadata if provided
    if !metadata.is_empty() {
        let metadata_path = format!("{}/metadata/{}.json", mount_point, doc_id);
        fs::create_dir_all(format!("{}/metadata", mount_point))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        
        let metadata_json = serde_json::to_string(&metadata)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        
        fs::write(&metadata_path, metadata_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
    }
    
    // Use vexctl to index the document
    let output = Command::new("vexctl")
        .args(&["index", "--file", &doc_path, "--id", &doc_id])
        .current_dir(mount_point)
        .output()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to execute vexctl: {}", e)
        ))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("vexctl indexing failed: {}", stderr)
        ));
    }
    
    Ok(doc_id)
}

#[pyfunction]
fn query(vector: Vec<f32>, top_k: usize) -> PyResult<Vec<String>> {
    let mount_point = unsafe {
        MOUNT_POINT.as_ref().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "VexFS not initialized. Call vexfs.init() first."
            )
        })?
    };
    
    // Write vector to temporary file
    let vector_str: Vec<String> = vector.iter().map(|f| f.to_string()).collect();
    let vector_data = vector_str.join(",");
    
    let temp_path = format!("{}/tmp/query_{}.vec", mount_point, rand::random::<u32>());
    fs::create_dir_all(format!("{}/tmp", mount_point))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
    
    fs::write(&temp_path, vector_data)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
    
    // Execute search using vexctl
    let output = Command::new("vexctl")
        .args(&[
            "search",
            "--vector-file", &temp_path,
            "--top-k", &top_k.to_string(),
            "--format", "json"
        ])
        .current_dir(mount_point)
        .output()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to execute vexctl: {}", e)
        ))?;
    
    // Clean up temporary file
    let _ = fs::remove_file(&temp_path);
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("vexctl search failed: {}", stderr)
        ));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let results: Vec<String> = serde_json::from_str(&stdout)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Failed to parse search results: {}", e)
        ))?;
    
    Ok(results)
}

#[pyfunction]
fn delete(id: String) -> PyResult<()> {
    let mount_point = unsafe {
        MOUNT_POINT.as_ref().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "VexFS not initialized. Call vexfs.init() first."
            )
        })?
    };
    
    // Remove document files
    let doc_path = format!("{}/documents/{}.txt", mount_point, id);
    let metadata_path = format!("{}/metadata/{}.json", mount_point, id);
    
    let _ = fs::remove_file(&doc_path);
    let _ = fs::remove_file(&metadata_path);
    
    // Remove from index using vexctl
    let output = Command::new("vexctl")
        .args(&["delete", "--id", &id])
        .current_dir(mount_point)
        .output()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to execute vexctl: {}", e)
        ))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("vexctl delete failed: {}", stderr)
        ));
    }
    
    Ok(())
}

#[pymodule]
fn vexfs(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(init, m)?)?;
    m.add_function(wrap_pyfunction!(add, m)?)?;
    m.add_function(wrap_pyfunction!(query, m)?)?;
    m.add_function(wrap_pyfunction!(delete, m)?)?;
    Ok(())
}