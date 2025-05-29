// Simple test to check ANNS module compilation


use core::mem;

// Import the ANNS types
mod ondisk;
mod anns;

use anns::*;

fn main() {
    // Test struct sizes
    println!("AnnsIndexHeader size: {} bytes", mem::size_of::<AnnsIndexHeader>());
    println!("HnswParams size: {} bytes", mem::size_of::<HnswParams>());
    println!("AnnsIndex size: {} bytes", mem::size_of::<AnnsIndex>());
    
    // Test basic functionality
    let params = HnswParams::default();
    let index = AnnsIndex::new(params, 128, ondisk::VectorDataType::Float32);
    
    println!("ANNS module test completed successfully!");
}