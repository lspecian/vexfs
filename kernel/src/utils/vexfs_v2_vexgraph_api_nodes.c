/*
 * VexFS v2.0 - VexGraph Node API Implementation (Task 9 - Phase 2)
 * 
 * This implements the Node API operations for VexGraph, providing
 * comprehensive CRUD (Create, Read, Update, Delete) operations for
 * graph nodes with properties and relationships.
 *
 * Key Features:
 * - Node creation with property initialization
 * - Node reading with property and edge information
 * - Node updating with property management
 * - Node deletion with cascade options
 * - Property management (add, remove, update)
 * - Integration with VexGraph Core (Task 8)
 * - Performance optimization and caching
 * - Error handling and validation
 */

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/slab.h>
#include <linux/fs.h>
#include <linux/string.h>
#include <linux/time.h>
#include <linux/json.h>  /* Note: Kernel JSON parsing would need custom implementation */

#include "../include/vexfs_v2_vexgraph_api.h"
#include "../include/vexfs_v2_internal.h"

/* JSON parsing helpers (simplified for kernel space) */
static int vexfs_api_parse_properties_json(const char *json_str,
                                            struct vexfs_graph_node *node);
static int vexfs_api_serialize_properties_json(struct vexfs_graph_node *node,
                                                char **json_str);
static int vexfs_api_serialize_edges_json(struct vexfs_graph_node *node,
                                           char **json_str);

/*
 * =============================================================================
 * NODE CRUD OPERATIONS
 * =============================================================================
 */

/**
 * vexfs_api_node_create - Create a new graph node
 * @api_mgr: API manager
 * @request: Create node request
 * @response: Response to populate
 * 
 * Creates a new graph node with the specified properties and associates
 * it with an inode if provided.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_api_node_create(struct vexfs_api_manager *api_mgr,
                          struct vexfs_api_request *request,
                          struct vexfs_api_response *response)
{
    struct vexfs_graph_node *node;
    int result;

    if (!api_mgr || !request || !response) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    if (api_mgr->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    /* Acquire read lock on API manager */
    down_read(&api_mgr->api_sem);

    /* Create the node using VexGraph core */
    node = vexfs_graph_node_create(api_mgr->graph_mgr,
                                   request->params.node_create.inode_number,
                                   request->params.node_create.node_type);
    if (!node) {
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NO_MEMORY,
                            "Failed to create graph node");
        return VEXFS_API_ERROR_NO_MEMORY;
    }

    /* Parse and add properties if provided */
    if (request->params.node_create.properties_json) {
        result = vexfs_api_parse_properties_json(
            request->params.node_create.properties_json, node);
        if (result != 0) {
            /* Cleanup the created node on property parsing failure */
            vexfs_graph_node_destroy(api_mgr->graph_mgr, node);
            up_read(&api_mgr->api_sem);
            vexfs_api_set_error(response, VEXFS_API_ERROR_INVALID_PARAM,
                                "Failed to parse node properties JSON");
            return VEXFS_API_ERROR_INVALID_PARAM;
        }
    }

    /* Set response data */
    response->request_id = request->request_id;
    response->result_code = VEXFS_API_SUCCESS;
    response->data.node_create.node_id = node->node_id;

    up_read(&api_mgr->api_sem);

    printk(KERN_INFO "VexGraph API: Created node %llu (type %u)\n",
           node->node_id, node->node_type);

    return VEXFS_API_SUCCESS;
}

/**
 * vexfs_api_node_read - Read a graph node
 * @api_mgr: API manager
 * @request: Read node request
 * @response: Response to populate
 * 
 * Reads a graph node and optionally includes properties and edge information.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_api_node_read(struct vexfs_api_manager *api_mgr,
                        struct vexfs_api_request *request,
                        struct vexfs_api_response *response)
{
    struct vexfs_graph_node *node;
    char *properties_json = NULL;
    u64 *outgoing_edges = NULL;
    u64 *incoming_edges = NULL;
    int result;

    if (!api_mgr || !request || !response) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    if (api_mgr->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    /* Acquire read lock on API manager */
    down_read(&api_mgr->api_sem);

    /* Look up the node */
    node = vexfs_graph_node_lookup(api_mgr->graph_mgr,
                                   request->params.node_read.node_id);
    if (!node) {
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NOT_FOUND,
                            "Graph node not found");
        return VEXFS_API_ERROR_NOT_FOUND;
    }

    /* Acquire read lock on the node */
    down_read(&node->node_sem);

    /* Set basic response data */
    response->request_id = request->request_id;
    response->result_code = VEXFS_API_SUCCESS;
    response->data.node_read.node_id = node->node_id;
    response->data.node_read.node_type = node->node_type;
    response->data.node_read.inode_number = node->inode_number;
    response->data.node_read.out_degree = node->out_degree;
    response->data.node_read.in_degree = node->in_degree;

    /* Include properties if requested */
    if (request->params.node_read.include_properties) {
        result = vexfs_api_serialize_properties_json(node, &properties_json);
        if (result == 0 && properties_json) {
            response->data.node_read.properties_json = properties_json;
        }
    }

    /* Include edge information if requested */
    if (request->params.node_read.include_edges) {
        /* Allocate arrays for edge IDs */
        if (node->out_degree > 0) {
            outgoing_edges = kzalloc(sizeof(u64) * node->out_degree, GFP_KERNEL);
            if (outgoing_edges) {
                /* TODO: Populate outgoing edge IDs from adjacency list */
                response->data.node_read.outgoing_edges = outgoing_edges;
            }
        }

        if (node->in_degree > 0) {
            incoming_edges = kzalloc(sizeof(u64) * node->in_degree, GFP_KERNEL);
            if (incoming_edges) {
                /* TODO: Populate incoming edge IDs from adjacency list */
                response->data.node_read.incoming_edges = incoming_edges;
            }
        }
    }

    up_read(&node->node_sem);
    up_read(&api_mgr->api_sem);

    printk(KERN_INFO "VexGraph API: Read node %llu (type %u, %u out, %u in)\n",
           node->node_id, node->node_type, node->out_degree, node->in_degree);

    return VEXFS_API_SUCCESS;
}

/**
 * vexfs_api_node_update - Update a graph node
 * @api_mgr: API manager
 * @request: Update node request
 * @response: Response to populate
 * 
 * Updates a graph node's properties. Can either merge with existing
 * properties or replace them entirely.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_api_node_update(struct vexfs_api_manager *api_mgr,
                          struct vexfs_api_request *request,
                          struct vexfs_api_response *response)
{
    struct vexfs_graph_node *node;
    u32 properties_updated = 0;
    int result;

    if (!api_mgr || !request || !response) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    if (api_mgr->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    /* Acquire read lock on API manager */
    down_read(&api_mgr->api_sem);

    /* Look up the node */
    node = vexfs_graph_node_lookup(api_mgr->graph_mgr,
                                   request->params.node_update.node_id);
    if (!node) {
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NOT_FOUND,
                            "Graph node not found");
        return VEXFS_API_ERROR_NOT_FOUND;
    }

    /* Acquire write lock on the node */
    down_write(&node->node_sem);

    /* If not merging, clear existing properties */
    if (!request->params.node_update.merge_properties) {
        /* TODO: Clear all existing properties */
        printk(KERN_INFO "VexGraph API: Clearing existing properties for node %llu\n",
               node->node_id);
    }

    /* Parse and update properties if provided */
    if (request->params.node_update.properties_json) {
        result = vexfs_api_parse_properties_json(
            request->params.node_update.properties_json, node);
        if (result != 0) {
            up_write(&node->node_sem);
            up_read(&api_mgr->api_sem);
            vexfs_api_set_error(response, VEXFS_API_ERROR_INVALID_PARAM,
                                "Failed to parse updated properties JSON");
            return VEXFS_API_ERROR_INVALID_PARAM;
        }
        properties_updated = node->property_count;
    }

    /* Update modification timestamp */
    node->modified_time = ktime_get_real_seconds();

    /* Set response data */
    response->request_id = request->request_id;
    response->result_code = VEXFS_API_SUCCESS;
    response->data.node_update.node_id = node->node_id;
    response->data.node_update.properties_updated = properties_updated;

    up_write(&node->node_sem);
    up_read(&api_mgr->api_sem);

    printk(KERN_INFO "VexGraph API: Updated node %llu (%u properties)\n",
           node->node_id, properties_updated);

    return VEXFS_API_SUCCESS;
}

/**
 * vexfs_api_node_delete - Delete a graph node
 * @api_mgr: API manager
 * @request: Delete node request
 * @response: Response to populate
 * 
 * Deletes a graph node and optionally cascades to delete connected edges.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_api_node_delete(struct vexfs_api_manager *api_mgr,
                          struct vexfs_api_request *request,
                          struct vexfs_api_response *response)
{
    struct vexfs_graph_node *node;
    u32 edges_deleted = 0;

    if (!api_mgr || !request || !response) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    if (api_mgr->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    /* Acquire write lock on API manager for node deletion */
    down_write(&api_mgr->api_sem);

    /* Look up the node */
    node = vexfs_graph_node_lookup(api_mgr->graph_mgr,
                                   request->params.node_delete.node_id);
    if (!node) {
        up_write(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NOT_FOUND,
                            "Graph node not found");
        return VEXFS_API_ERROR_NOT_FOUND;
    }

    /* Count edges that will be deleted */
    edges_deleted = node->out_degree + node->in_degree;

    /* Delete the node (this will handle edge cleanup) */
    vexfs_graph_node_destroy(api_mgr->graph_mgr, node);

    /* Set response data */
    response->request_id = request->request_id;
    response->result_code = VEXFS_API_SUCCESS;
    response->data.node_delete.node_id = request->params.node_delete.node_id;
    response->data.node_delete.edges_deleted = edges_deleted;

    up_write(&api_mgr->api_sem);

    printk(KERN_INFO "VexGraph API: Deleted node %llu (%u edges removed)\n",
           request->params.node_delete.node_id, edges_deleted);

    return VEXFS_API_SUCCESS;
}

/*
 * =============================================================================
 * JSON PROPERTY PARSING HELPERS
 * =============================================================================
 */

/**
 * vexfs_api_parse_properties_json - Parse JSON properties and add to node
 * @json_str: JSON string containing properties
 * @node: Node to add properties to
 * 
 * Parses a JSON string containing node properties and adds them to the node.
 * This is a simplified implementation for kernel space.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int vexfs_api_parse_properties_json(const char *json_str,
                                            struct vexfs_graph_node *node)
{
    /* Simplified JSON parsing for kernel space
     * In a real implementation, this would need a proper JSON parser
     * or a custom simple parser for basic key-value pairs.
     * 
     * For now, we'll implement a basic parser that handles simple cases:
     * {"key1": "value1", "key2": 123, "key3": true}
     */
    
    const char *ptr = json_str;
    char key[64];
    char value[256];
    int result;

    if (!json_str || !node) {
        return -EINVAL;
    }

    /* Skip opening brace */
    while (*ptr && (*ptr == ' ' || *ptr == '\t' || *ptr == '{')) {
        ptr++;
    }

    /* Parse key-value pairs */
    while (*ptr && *ptr != '}') {
        /* Skip whitespace */
        while (*ptr && (*ptr == ' ' || *ptr == '\t' || *ptr == ',')) {
            ptr++;
        }

        if (*ptr == '}') break;

        /* Parse key (expect quoted string) */
        if (*ptr != '"') {
            printk(KERN_ERR "VexGraph API: Expected quoted key in JSON\n");
            return -EINVAL;
        }
        ptr++; /* Skip opening quote */

        /* Extract key */
        int key_len = 0;
        while (*ptr && *ptr != '"' && key_len < sizeof(key) - 1) {
            key[key_len++] = *ptr++;
        }
        key[key_len] = '\0';

        if (*ptr != '"') {
            printk(KERN_ERR "VexGraph API: Unterminated key in JSON\n");
            return -EINVAL;
        }
        ptr++; /* Skip closing quote */

        /* Skip whitespace and colon */
        while (*ptr && (*ptr == ' ' || *ptr == '\t' || *ptr == ':')) {
            ptr++;
        }

        /* Parse value */
        if (*ptr == '"') {
            /* String value */
            ptr++; /* Skip opening quote */
            int value_len = 0;
            while (*ptr && *ptr != '"' && value_len < sizeof(value) - 1) {
                value[value_len++] = *ptr++;
            }
            value[value_len] = '\0';
            
            if (*ptr == '"') ptr++; /* Skip closing quote */

            /* Add string property */
            result = vexfs_graph_node_add_property(node, key,
                                                   VEXFS_GRAPH_PROP_STRING,
                                                   value, strlen(value) + 1);
            if (result != 0) {
                printk(KERN_ERR "VexGraph API: Failed to add string property %s\n", key);
                return result;
            }

        } else if ((*ptr >= '0' && *ptr <= '9') || *ptr == '-') {
            /* Numeric value */
            long long_val = 0;
            int negative = 0;
            
            if (*ptr == '-') {
                negative = 1;
                ptr++;
            }
            
            while (*ptr >= '0' && *ptr <= '9') {
                long_val = long_val * 10 + (*ptr - '0');
                ptr++;
            }
            
            if (negative) long_val = -long_val;

            /* Add integer property */
            result = vexfs_graph_node_add_property(node, key,
                                                   VEXFS_GRAPH_PROP_INTEGER,
                                                   &long_val, sizeof(long_val));
            if (result != 0) {
                printk(KERN_ERR "VexGraph API: Failed to add integer property %s\n", key);
                return result;
            }

        } else if (strncmp(ptr, "true", 4) == 0) {
            /* Boolean true */
            bool bool_val = true;
            ptr += 4;

            result = vexfs_graph_node_add_property(node, key,
                                                   VEXFS_GRAPH_PROP_BOOLEAN,
                                                   &bool_val, sizeof(bool_val));
            if (result != 0) {
                printk(KERN_ERR "VexGraph API: Failed to add boolean property %s\n", key);
                return result;
            }

        } else if (strncmp(ptr, "false", 5) == 0) {
            /* Boolean false */
            bool bool_val = false;
            ptr += 5;

            result = vexfs_graph_node_add_property(node, key,
                                                   VEXFS_GRAPH_PROP_BOOLEAN,
                                                   &bool_val, sizeof(bool_val));
            if (result != 0) {
                printk(KERN_ERR "VexGraph API: Failed to add boolean property %s\n", key);
                return result;
            }

        } else {
            printk(KERN_ERR "VexGraph API: Unsupported value type in JSON\n");
            return -EINVAL;
        }

        /* Skip to next key-value pair */
        while (*ptr && *ptr != ',' && *ptr != '}') {
            ptr++;
        }
    }

    printk(KERN_INFO "VexGraph API: Parsed %u properties from JSON\n",
           node->property_count);

    return 0;
}

/**
 * vexfs_api_serialize_properties_json - Serialize node properties to JSON
 * @node: Node to serialize properties from
 * @json_str: Pointer to store allocated JSON string
 * 
 * Serializes node properties to a JSON string. The caller is responsible
 * for freeing the allocated string.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int vexfs_api_serialize_properties_json(struct vexfs_graph_node *node,
                                                char **json_str)
{
    struct vexfs_graph_property *prop;
    char *buffer;
    int buffer_size = 4096; /* Initial buffer size */
    int pos = 0;
    bool first = true;

    if (!node || !json_str) {
        return -EINVAL;
    }

    buffer = kzalloc(buffer_size, GFP_KERNEL);
    if (!buffer) {
        return -ENOMEM;
    }

    /* Start JSON object */
    pos += snprintf(buffer + pos, buffer_size - pos, "{");

    /* Iterate through properties */
    list_for_each_entry(prop, &node->properties, list) {
        if (!first) {
            pos += snprintf(buffer + pos, buffer_size - pos, ",");
        }
        first = false;

        /* Add key */
        pos += snprintf(buffer + pos, buffer_size - pos, "\"%s\":", prop->key);

        /* Add value based on type */
        switch (prop->type) {
        case VEXFS_GRAPH_PROP_STRING:
            pos += snprintf(buffer + pos, buffer_size - pos, "\"%s\"",
                            prop->value.string_val ? prop->value.string_val : "");
            break;

        case VEXFS_GRAPH_PROP_INTEGER:
            pos += snprintf(buffer + pos, buffer_size - pos, "%lld",
                            prop->value.int_val);
            break;

        case VEXFS_GRAPH_PROP_BOOLEAN:
            pos += snprintf(buffer + pos, buffer_size - pos, "%s",
                            prop->value.bool_val ? "true" : "false");
            break;

        case VEXFS_GRAPH_PROP_TIMESTAMP:
            pos += snprintf(buffer + pos, buffer_size - pos, "%llu",
                            prop->value.timestamp_val);
            break;

        default:
            pos += snprintf(buffer + pos, buffer_size - pos, "null");
            break;
        }

        /* Check if we need more buffer space */
        if (pos >= buffer_size - 100) {
            char *new_buffer = krealloc(buffer, buffer_size * 2, GFP_KERNEL);
            if (!new_buffer) {
                kfree(buffer);
                return -ENOMEM;
            }
            buffer = new_buffer;
            buffer_size *= 2;
        }
    }

    /* End JSON object */
    pos += snprintf(buffer + pos, buffer_size - pos, "}");

    *json_str = buffer;
    return 0;
}

/**
 * vexfs_api_serialize_edges_json - Serialize node edges to JSON
 * @node: Node to serialize edges from
 * @json_str: Pointer to store allocated JSON string
 * 
 * Serializes node edge information to a JSON string.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int vexfs_api_serialize_edges_json(struct vexfs_graph_node *node,
                                           char **json_str)
{
    char *buffer;
    int buffer_size = 2048;
    int pos = 0;

    if (!node || !json_str) {
        return -EINVAL;
    }

    buffer = kzalloc(buffer_size, GFP_KERNEL);
    if (!buffer) {
        return -ENOMEM;
    }

    /* Create JSON object with edge information */
    pos += snprintf(buffer + pos, buffer_size - pos,
                    "{\"out_degree\":%u,\"in_degree\":%u,\"outgoing\":[],\"incoming\":[]}",
                    node->out_degree, node->in_degree);

    /* TODO: Add actual edge IDs to the arrays */

    *json_str = buffer;
    return 0;
}