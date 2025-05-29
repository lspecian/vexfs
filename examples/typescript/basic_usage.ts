import VexFSClient from 'vexfs-sdk';

async function main() {
  const client = new VexFSClient();
  
  try {
    // Add a document
    const docId = await client.add("Hello world", { type: "greeting", lang: "en" });
    console.log(`Added document with ID: ${docId}`);
    
    // Query (placeholder - will need actual vector)
    const results = await client.query([0.1, 0.2, 0.3], 5);
    console.log(`Query results:`, results);
    
    // Delete document
    await client.delete(docId);
    console.log("Document deleted");
  } catch (error) {
    console.error("Error:", error);
  }
}

main().catch(console.error);