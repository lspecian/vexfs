"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.VexFSClient = void 0;
class VexFSClient {
    constructor(config = {}) {
        this.baseUrl = config.baseUrl || 'http://localhost:8080';
        this.timeout = config.timeout || 30000;
    }
    async add(text, metadata = {}) {
        // TODO: Implement REST API call
        throw new Error('Not implemented yet');
    }
    async query(vector, topK = 10) {
        // TODO: Implement REST API call
        throw new Error('Not implemented yet');
    }
    async delete(id) {
        // TODO: Implement REST API call
        throw new Error('Not implemented yet');
    }
}
exports.VexFSClient = VexFSClient;
exports.default = VexFSClient;
//# sourceMappingURL=client.js.map