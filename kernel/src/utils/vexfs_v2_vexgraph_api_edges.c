/*
 * VexFS v2.0 - VexGraph Edge API Implementation (Task 9 - Phase 2)
 * 
 * This implements the Edge API operations for VexGraph, providing
 * comprehensive CRUD (Create, Read, Update, Delete) operations for
 * graph edges with properties and relationships.
 *
 * Key Features:
 * - Edge creation with relationship types and weights
 * - Edge reading with property information
 * - Edge updating with weight and property management
 * - Edge deletion with relationship cleanup
 * - Property management for edges
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

#include "../include/vexfs_v2_vexgraph_api.h"
#include "../include/vexfs_v2_internal.h"

/* Edge property parsing helpers */
static int vexfs_api_parse_edge_properties_json(const char *json_str,
                                                 struct vexfs_graph_edge *edge);
static int vexfs_api_serialize_edge_properties_json(struct vexfs_graph_edge *edge,
                                                     char **json_str);

/*
 * =============================================================================
 * EDGE CRUD OPERATIONS
 * =============================================================================
 */

/**
 * vexfs_api_edge_create - Create a new graph edge
 * @api_mgr: API manager
 * @request: Create edge request
 * @response: Response to populate
 * 
 * Creates a new graph edge between two nodes with the specified type,
 * weight, and properties.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_api_edge_create(struct vexfs_api_manager *api_mgr,
                          struct vexfs_api_request *request,
                          struct vexfs_api_response *response)
{
    struct vexfs_graph_edge *edge;
    struct vexfs_graph_node *source_node, *target_node;
    int result;

    if (!api_mgr || !request || !response) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    if (api_mgr->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    /* Acquire read lock on API manager */
    down_read(&api_mgr->api_sem);

    /* Verify source and target nodes exist */
    source_node = vexfs_graph_node_lookup(api_mgr->graph_mgr,
                                           request->params.edge_create.source_id);
    if (!source_node) {
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NOT_FOUND,
                            "Source node not found");
        return VEXFS_API_ERROR_NOT_FOUND;
    }

    target_node = vexfs_graph_node_lookup(api_mgr->graph_mgr,
                                           request->params.edge_create.target_id);
    if (!target_node) {
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NOT_FOUND,
                            "Target node not found");
        return VEXFS_API_ERROR_NOT_FOUND;
    }

    /* Create the edge using VexGraph core */
    edge = vexfs_graph_edge_create(api_mgr->graph_mgr,
                                   request->params.edge_create.source_id,
                                   request->params.edge_create.target_id,
                                   request->params.edge_create.edge_type,
                                   request->params.edge_create.weight);
    if (!edge) {
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NO_MEMORY,
                            "Failed to create graph edge");
        return VEXFS_API_ERROR_NO_MEMORY;
    }

    /* Parse and add properties if provided */
    if (request->params.edge_create.properties_json) {
        result = vexfs_api_parse_edge_properties_json(
            request->params.edge_create.properties_json, edge);
        if (result != 0) {
            /* Cleanup the created edge on property parsing failure */
            vexfs_graph_edge_destroy(api_mgr->graph_mgr, edge);
            up_read(&api_mgr->api_sem);
            vexfs_api_set_error(response, VEXFS_API_ERROR_INVALID_PARAM,
                                "Failed to parse edge properties JSON");
            return VEXFS_API_ERROR_INVALID_PARAM;
        }
    }

    /* Set response data */
    response->request_id = request->request_id;
    response->result_code = VEXFS_API_SUCCESS;
    response->data.edge_create.edge_id = edge->edge_id;

    up_read(&api_mgr->api_sem);

    printk(KERN_INFO "VexGraph API: Created edge %llu (%llu -> %llu, type %u, weight %u)\n",
           edge->edge_id, edge->source_node_id, edge->target_node_id,
           edge->edge_type, edge->weight);

    return VEXFS_API_SUCCESS;
}

/**
 * vexfs_api_edge_read - Read a graph edge
 * @api_mgr: API manager
 * @request: Read edge request
 * @response: Response to populate
 * 
 * Reads a graph edge and optionally includes property information.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_api_edge_read(struct vexfs_api_manager *api_mgr,
                        struct vexfs_api_request *request,
                        struct vexfs_api_response *response)
{
    struct vexfs_graph_edge *edge;
    char *properties_json = NULL;
    int result;

    if (!api_mgr || !request || !response) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    if (api_mgr->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    /* Acquire read lock on API manager */
    down_read(&api_mgr->api_sem);

    /* Look up the edge */
    edge = vexfs_graph_edge_lookup(api_mgr->graph_mgr,
                                   request->params.edge_read.edge_id);
    if (!edge) {
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NOT_FOUND,
                            "Graph edge not found");
        return VEXFS_API_ERROR_NOT_FOUND;
    }

    /* Acquire read lock on the edge */
    spin_lock(&edge->edge_lock);

    /* Set basic response data */
    response->request_id = request->request_id;
    response->result_code = VEXFS_API_SUCCESS;
    response->data.edge_read.edge_id = edge->edge_id;
    response->data.edge_read.source_id = edge->source_node_id;
    response->data.edge_read.target_id = edge->target_node_id;
    response->data.edge_read.edge_type = edge->edge_type;
    response->data.edge_read.weight = edge->weight;

    /* Include properties if requested */
    if (request->params.edge_read.include_properties) {
        result = vexfs_api_serialize_edge_properties_json(edge, &properties_json);
        if (result == 0 && properties_json) {
            response->data.edge_read.properties_json = properties_json;
        }
    }

    spin_unlock(&edge->edge_lock);
    up_read(&api_mgr->api_sem);

    printk(KERN_INFO "VexGraph API: Read edge %llu (%llu -> %llu, type %u, weight %u)\n",
           edge->edge_id, edge->source_node_id, edge->target_node_id,
           edge->edge_type, edge->weight);

    return VEXFS_API_SUCCESS;
}

/**
 * vexfs_api_edge_update - Update a graph edge
 * @api_mgr: API manager
 * @request: Update edge request
 * @response: Response to populate
 * 
 * Updates a graph edge's weight and properties.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_api_edge_update(struct vexfs_api_manager *api_mgr,
                          struct vexfs_api_request *request,
                          struct vexfs_api_response *response)
{
    struct vexfs_graph_edge *edge;
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

    /* Look up the edge */
    edge = vexfs_graph_edge_lookup(api_mgr->graph_mgr,
                                   request->params.edge_update.edge_id);
    if (!edge) {
        up_read(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NOT_FOUND,
                            "Graph edge not found");
        return VEXFS_API_ERROR_NOT_FOUND;
    }

    /* Acquire write lock on the edge */
    spin_lock(&edge->edge_lock);

    /* Update weight if provided */
    if (request->params.edge_update.weight != edge->weight) {
        edge->weight = request->params.edge_update.weight;
        printk(KERN_INFO "VexGraph API: Updated edge %llu weight to %u\n",
               edge->edge_id, edge->weight);
    }

    /* Parse and update properties if provided */
    if (request->params.edge_update.properties_json) {
        result = vexfs_api_parse_edge_properties_json(
            request->params.edge_update.properties_json, edge);
        if (result != 0) {
            spin_unlock(&edge->edge_lock);
            up_read(&api_mgr->api_sem);
            vexfs_api_set_error(response, VEXFS_API_ERROR_INVALID_PARAM,
                                "Failed to parse updated edge properties JSON");
            return VEXFS_API_ERROR_INVALID_PARAM;
        }
        properties_updated = edge->property_count;
    }

    /* Update modification timestamp */
    edge->modified_time = ktime_get_real_seconds();

    /* Set response data */
    response->request_id = request->request_id;
    response->result_code = VEXFS_API_SUCCESS;
    response->data.edge_update.edge_id = edge->edge_id;
    response->data.edge_update.properties_updated = properties_updated;

    spin_unlock(&edge->edge_lock);
    up_read(&api_mgr->api_sem);

    printk(KERN_INFO "VexGraph API: Updated edge %llu (%u properties)\n",
           edge->edge_id, properties_updated);

    return VEXFS_API_SUCCESS;
}

/**
 * vexfs_api_edge_delete - Delete a graph edge
 * @api_mgr: API manager
 * @request: Delete edge request
 * @response: Response to populate
 * 
 * Deletes a graph edge and removes it from the adjacency lists of
 * connected nodes.
 * 
 * Return: 0 on success, negative error code on failure
 */
int vexfs_api_edge_delete(struct vexfs_api_manager *api_mgr,
                          struct vexfs_api_request *request,
                          struct vexfs_api_response *response)
{
    struct vexfs_graph_edge *edge;
    u64 edge_id;

    if (!api_mgr || !request || !response) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    if (api_mgr->magic != VEXFS_VEXGRAPH_API_MAGIC) {
        return VEXFS_API_ERROR_INVALID_PARAM;
    }

    /* Acquire write lock on API manager for edge deletion */
    down_write(&api_mgr->api_sem);

    /* Look up the edge */
    edge = vexfs_graph_edge_lookup(api_mgr->graph_mgr,
                                   request->params.edge_delete.edge_id);
    if (!edge) {
        up_write(&api_mgr->api_sem);
        vexfs_api_set_error(response, VEXFS_API_ERROR_NOT_FOUND,
                            "Graph edge not found");
        return VEXFS_API_ERROR_NOT_FOUND;
    }

    edge_id = edge->edge_id;

    /* Delete the edge (this will handle adjacency list cleanup) */
    vexfs_graph_edge_destroy(api_mgr->graph_mgr, edge);

    /* Set response data */
    response->request_id = request->request_id;
    response->result_code = VEXFS_API_SUCCESS;
    response->data.edge_delete.edge_id = edge_id;

    up_write(&api_mgr->api_sem);

    printk(KERN_INFO "VexGraph API: Deleted edge %llu\n", edge_id);

    return VEXFS_API_SUCCESS;
}

/*
 * =============================================================================
 * EDGE PROPERTY PARSING HELPERS
 * =============================================================================
 */

/**
 * vexfs_api_parse_edge_properties_json - Parse JSON properties and add to edge
 * @json_str: JSON string containing properties
 * @edge: Edge to add properties to
 * 
 * Parses a JSON string containing edge properties and adds them to the edge.
 * This is a simplified implementation for kernel space.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int vexfs_api_parse_edge_properties_json(const char *json_str,
                                                 struct vexfs_graph_edge *edge)
{
    /* Simplified JSON parsing for kernel space
     * Similar to node property parsing but for edges
     */
    
    const char *ptr = json_str;
    char key[64];
    char value[256];
    int result;

    if (!json_str || !edge) {
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
            printk(KERN_ERR "VexGraph API: Expected quoted key in edge JSON\n");
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
            printk(KERN_ERR "VexGraph API: Unterminated key in edge JSON\n");
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
            result = vexfs_graph_edge_add_property(edge, key,
                                                   VEXFS_GRAPH_PROP_STRING,
                                                   value, strlen(value) + 1);
            if (result != 0) {
                printk(KERN_ERR "VexGraph API: Failed to add edge string property %s\n", key);
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
            result = vexfs_graph_edge_add_property(edge, key,
                                                   VEXFS_GRAPH_PROP_INTEGER,
                                                   &long_val, sizeof(long_val));
            if (result != 0) {
                printk(KERN_ERR "VexGraph API: Failed to add edge integer property %s\n", key);
                return result;
            }

        } else if (strncmp(ptr, "true", 4) == 0) {
            /* Boolean true */
            bool bool_val = true;
            ptr += 4;

            result = vexfs_graph_edge_add_property(edge, key,
                                                   VEXFS_GRAPH_PROP_BOOLEAN,
                                                   &bool_val, sizeof(bool_val));
            if (result != 0) {
                printk(KERN_ERR "VexGraph API: Failed to add edge boolean property %s\n", key);
                return result;
            }

        } else if (strncmp(ptr, "false", 5) == 0) {
            /* Boolean false */
            bool bool_val = false;
            ptr += 5;

            result = vexfs_graph_edge_add_property(edge, key,
                                                   VEXFS_GRAPH_PROP_BOOLEAN,
                                                   &bool_val, sizeof(bool_val));
            if (result != 0) {
                printk(KERN_ERR "VexGraph API: Failed to add edge boolean property %s\n", key);
                return result;
            }

        } else {
            printk(KERN_ERR "VexGraph API: Unsupported value type in edge JSON\n");
            return -EINVAL;
        }

        /* Skip to next key-value pair */
        while (*ptr && *ptr != ',' && *ptr != '}') {
            ptr++;
        }
    }

    printk(KERN_INFO "VexGraph API: Parsed %u properties from edge JSON\n",
           edge->property_count);

    return 0;
}

/**
 * vexfs_api_serialize_edge_properties_json - Serialize edge properties to JSON
 * @edge: Edge to serialize properties from
 * @json_str: Pointer to store allocated JSON string
 * 
 * Serializes edge properties to a JSON string. The caller is responsible
 * for freeing the allocated string.
 * 
 * Return: 0 on success, negative error code on failure
 */
static int vexfs_api_serialize_edge_properties_json(struct vexfs_graph_edge *edge,
                                                     char **json_str)
{
    struct vexfs_graph_property *prop;
    char *buffer;
    int buffer_size = 2048; /* Initial buffer size */
    int pos = 0;
    bool first = true;

    if (!edge || !json_str) {
        return -EINVAL;
    }

    buffer = kzalloc(buffer_size, GFP_KERNEL);
    if (!buffer) {
        return -ENOMEM;
    }

    /* Start JSON object */
    pos += snprintf(buffer + pos, buffer_size - pos, "{");

    /* Iterate through properties */
    list_for_each_entry(prop, &edge->properties, list) {
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