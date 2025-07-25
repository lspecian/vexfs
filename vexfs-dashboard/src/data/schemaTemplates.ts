import type { SchemaTemplate } from '../types/schema';

export const schemaTemplates: SchemaTemplate[] = [
  {
    id: 'filesystem',
    name: 'File System Schema',
    description: 'Schema for representing file system structures with files, directories, and relationships',
    category: 'System',
    nodeTypes: [
      {
        id: 'file',
        name: 'File',
        displayName: 'File',
        description: 'Represents a file in the filesystem',
        icon: 'description',
        color: '#2196f3',
        properties: [
          {
            name: 'name',
            type: 'String',
            constraints: [{ type: 'required' }],
            description: 'File name including extension',
            examples: ['document.txt', 'image.jpg', 'script.js'],
          },
          {
            name: 'size',
            type: 'Number',
            constraints: [{ type: 'required', minValue: 0 }],
            description: 'File size in bytes',
            examples: [1024, 2048576, 0],
          },
          {
            name: 'mimeType',
            type: 'String',
            constraints: [{ type: 'optional' }],
            description: 'MIME type of the file',
            examples: ['text/plain', 'image/jpeg', 'application/javascript'],
          },
          {
            name: 'lastModified',
            type: 'Date',
            constraints: [{ type: 'required' }],
            description: 'Last modification timestamp',
          },
          {
            name: 'permissions',
            type: 'String',
            constraints: [{ type: 'optional', pattern: '^[rwx-]{9}$' }],
            description: 'Unix-style permissions',
            examples: ['rw-r--r--', 'rwxr-xr-x'],
          },
        ],
        validationRules: [],
        indexHints: ['name', 'mimeType'],
      },
      {
        id: 'directory',
        name: 'Directory',
        displayName: 'Directory',
        description: 'Represents a directory in the filesystem',
        icon: 'folder',
        color: '#ff9800',
        properties: [
          {
            name: 'name',
            type: 'String',
            constraints: [{ type: 'required' }],
            description: 'Directory name',
            examples: ['Documents', 'Pictures', 'src'],
          },
          {
            name: 'path',
            type: 'String',
            constraints: [{ type: 'required' }],
            description: 'Full path to the directory',
            examples: ['/home/user/Documents', '/var/log', '/usr/local/bin'],
          },
          {
            name: 'permissions',
            type: 'String',
            constraints: [{ type: 'optional', pattern: '^[rwx-]{9}$' }],
            description: 'Unix-style permissions',
            examples: ['rwxr-xr-x', 'rwx------'],
          },
        ],
        validationRules: [],
        indexHints: ['name', 'path'],
      },
    ],
    edgeTypes: [
      {
        id: 'contains',
        name: 'Contains',
        displayName: 'Contains',
        description: 'Directory contains file or subdirectory',
        directionality: 'directed',
        allowedSourceTypes: ['Directory'],
        allowedTargetTypes: ['File', 'Directory'],
        cardinality: 'one-to-many',
        properties: [],
        weightConstraints: {
          required: false,
          min: 0,
          max: 1,
          defaultValue: 1,
        },
        validationRules: [],
      },
      {
        id: 'symlink',
        name: 'Symlink',
        displayName: 'Symbolic Link',
        description: 'Symbolic link between files or directories',
        directionality: 'directed',
        allowedSourceTypes: ['File', 'Directory'],
        allowedTargetTypes: ['File', 'Directory'],
        cardinality: 'many-to-many',
        properties: [
          {
            name: 'linkType',
            type: 'String',
            constraints: [{ type: 'required', enum: ['hard', 'soft'] }],
            description: 'Type of symbolic link',
          },
        ],
        weightConstraints: {
          required: false,
          min: 0,
          max: 1,
          defaultValue: 1,
        },
        validationRules: [],
      },
    ],
    sampleData: {
      nodes: [
        { id: 'root', type: 'Directory', properties: { name: '/', path: '/' } },
        { id: 'home', type: 'Directory', properties: { name: 'home', path: '/home' } },
        { id: 'user', type: 'Directory', properties: { name: 'user', path: '/home/user' } },
        { id: 'readme', type: 'File', properties: { name: 'README.md', size: 1024, mimeType: 'text/markdown' } },
      ],
      edges: [
        { id: 'e1', source: 'root', target: 'home', type: 'Contains' },
        { id: 'e2', source: 'home', target: 'user', type: 'Contains' },
        { id: 'e3', source: 'user', target: 'readme', type: 'Contains' },
      ],
    },
  },
  {
    id: 'social_network',
    name: 'Social Network Schema',
    description: 'Schema for social networking applications with users, posts, and relationships',
    category: 'Social',
    nodeTypes: [
      {
        id: 'user',
        name: 'User',
        displayName: 'User',
        description: 'Represents a user in the social network',
        icon: 'person',
        color: '#4caf50',
        properties: [
          {
            name: 'username',
            type: 'String',
            constraints: [
              { type: 'required' },
              { type: 'unique' },
              { type: 'optional', minLength: 3, maxLength: 30 }
            ],
            description: 'Unique username',
            examples: ['john_doe', 'alice_smith', 'bob123'],
          },
          {
            name: 'email',
            type: 'String',
            constraints: [
              { type: 'required' },
              { type: 'unique' },
              { type: 'optional', pattern: '^[^@]+@[^@]+\\.[^@]+$' }
            ],
            description: 'User email address',
            examples: ['john@example.com', 'alice@company.org'],
          },
          {
            name: 'displayName',
            type: 'String',
            constraints: [{ type: 'required', maxLength: 100 }],
            description: 'Display name for the user',
            examples: ['John Doe', 'Alice Smith'],
          },
          {
            name: 'bio',
            type: 'String',
            constraints: [{ type: 'optional', maxLength: 500 }],
            description: 'User biography',
          },
          {
            name: 'joinDate',
            type: 'Date',
            constraints: [{ type: 'required' }],
            description: 'Date when user joined',
          },
          {
            name: 'isVerified',
            type: 'Boolean',
            constraints: [{ type: 'optional', defaultValue: false }],
            description: 'Whether the user is verified',
          },
        ],
        validationRules: [],
        indexHints: ['username', 'email'],
      },
      {
        id: 'post',
        name: 'Post',
        displayName: 'Post',
        description: 'Represents a post in the social network',
        icon: 'article',
        color: '#9c27b0',
        properties: [
          {
            name: 'content',
            type: 'String',
            constraints: [{ type: 'required', maxLength: 2000 }],
            description: 'Post content',
          },
          {
            name: 'createdAt',
            type: 'Date',
            constraints: [{ type: 'required' }],
            description: 'Post creation timestamp',
          },
          {
            name: 'isPublic',
            type: 'Boolean',
            constraints: [{ type: 'optional', defaultValue: true }],
            description: 'Whether the post is public',
          },
          {
            name: 'tags',
            type: 'Array',
            constraints: [{ type: 'optional' }],
            description: 'Post tags',
            examples: [['technology', 'programming'], ['travel', 'photography']],
          },
        ],
        validationRules: [],
        indexHints: ['createdAt', 'tags'],
      },
    ],
    edgeTypes: [
      {
        id: 'follows',
        name: 'Follows',
        displayName: 'Follows',
        description: 'User follows another user',
        directionality: 'directed',
        allowedSourceTypes: ['User'],
        allowedTargetTypes: ['User'],
        cardinality: 'many-to-many',
        properties: [
          {
            name: 'followedAt',
            type: 'Date',
            constraints: [{ type: 'required' }],
            description: 'When the follow relationship was created',
          },
        ],
        weightConstraints: {
          required: false,
          min: 0,
          max: 1,
          defaultValue: 1,
        },
        validationRules: [],
      },
      {
        id: 'authored',
        name: 'Authored',
        displayName: 'Authored',
        description: 'User authored a post',
        directionality: 'directed',
        allowedSourceTypes: ['User'],
        allowedTargetTypes: ['Post'],
        cardinality: 'one-to-many',
        properties: [],
        weightConstraints: {
          required: false,
          min: 0,
          max: 1,
          defaultValue: 1,
        },
        validationRules: [],
      },
      {
        id: 'likes',
        name: 'Likes',
        displayName: 'Likes',
        description: 'User likes a post',
        directionality: 'directed',
        allowedSourceTypes: ['User'],
        allowedTargetTypes: ['Post'],
        cardinality: 'many-to-many',
        properties: [
          {
            name: 'likedAt',
            type: 'Date',
            constraints: [{ type: 'required' }],
            description: 'When the like was created',
          },
        ],
        weightConstraints: {
          required: false,
          min: 0,
          max: 1,
          defaultValue: 1,
        },
        validationRules: [],
      },
    ],
  },
  {
    id: 'knowledge_graph',
    name: 'Knowledge Graph Schema',
    description: 'Schema for knowledge graphs with entities, concepts, and semantic relationships',
    category: 'Knowledge',
    nodeTypes: [
      {
        id: 'entity',
        name: 'Entity',
        displayName: 'Entity',
        description: 'Represents an entity in the knowledge graph',
        icon: 'account_circle',
        color: '#f44336',
        properties: [
          {
            name: 'label',
            type: 'String',
            constraints: [{ type: 'required' }],
            description: 'Human-readable label',
            examples: ['Albert Einstein', 'Theory of Relativity', 'Physics'],
          },
          {
            name: 'type',
            type: 'String',
            constraints: [{ type: 'required', enum: ['Person', 'Concept', 'Organization', 'Location', 'Event'] }],
            description: 'Type of entity',
          },
          {
            name: 'description',
            type: 'String',
            constraints: [{ type: 'optional', maxLength: 1000 }],
            description: 'Entity description',
          },
          {
            name: 'aliases',
            type: 'Array',
            constraints: [{ type: 'optional' }],
            description: 'Alternative names for the entity',
            examples: [['Einstein', 'A. Einstein'], ['E=mc²', 'Mass-energy equivalence']],
          },
        ],
        validationRules: [],
        indexHints: ['label', 'type'],
      },
      {
        id: 'concept',
        name: 'Concept',
        displayName: 'Concept',
        description: 'Represents an abstract concept',
        icon: 'lightbulb',
        color: '#ff5722',
        properties: [
          {
            name: 'name',
            type: 'String',
            constraints: [{ type: 'required' }],
            description: 'Concept name',
            examples: ['Quantum Mechanics', 'Machine Learning', 'Democracy'],
          },
          {
            name: 'domain',
            type: 'String',
            constraints: [{ type: 'required' }],
            description: 'Domain or field of the concept',
            examples: ['Physics', 'Computer Science', 'Political Science'],
          },
          {
            name: 'definition',
            type: 'String',
            constraints: [{ type: 'optional', maxLength: 2000 }],
            description: 'Formal definition of the concept',
          },
        ],
        validationRules: [],
        indexHints: ['name', 'domain'],
      },
    ],
    edgeTypes: [
      {
        id: 'related_to',
        name: 'RelatedTo',
        displayName: 'Related To',
        description: 'General relationship between entities',
        directionality: 'undirected',
        allowedSourceTypes: ['Entity', 'Concept'],
        allowedTargetTypes: ['Entity', 'Concept'],
        cardinality: 'many-to-many',
        properties: [
          {
            name: 'relationshipType',
            type: 'String',
            constraints: [{ type: 'optional' }],
            description: 'Type of relationship',
            examples: ['colleague', 'influenced_by', 'part_of'],
          },
          {
            name: 'strength',
            type: 'Number',
            constraints: [{ type: 'optional', minValue: 0, maxValue: 1 }],
            description: 'Strength of the relationship',
          },
        ],
        weightConstraints: {
          required: true,
          min: 0,
          max: 1,
          defaultValue: 0.5,
        },
        validationRules: [],
      },
    ],
  },
  {
    id: 'workflow',
    name: 'Workflow Schema',
    description: 'Schema for workflow management with tasks, dependencies, and execution flow',
    category: 'Process',
    nodeTypes: [
      {
        id: 'task',
        name: 'Task',
        displayName: 'Task',
        description: 'Represents a task in the workflow',
        icon: 'task',
        color: '#3f51b5',
        properties: [
          {
            name: 'name',
            type: 'String',
            constraints: [{ type: 'required', maxLength: 200 }],
            description: 'Task name',
            examples: ['Data Processing', 'Send Email', 'Generate Report'],
          },
          {
            name: 'status',
            type: 'String',
            constraints: [{ type: 'required', enum: ['pending', 'running', 'completed', 'failed', 'cancelled'] }],
            description: 'Current task status',
          },
          {
            name: 'priority',
            type: 'String',
            constraints: [{ type: 'optional', enum: ['low', 'medium', 'high', 'critical'] }],
            description: 'Task priority',
          },
          {
            name: 'estimatedDuration',
            type: 'Number',
            constraints: [{ type: 'optional', minValue: 0 }],
            description: 'Estimated duration in minutes',
          },
          {
            name: 'assignee',
            type: 'String',
            constraints: [{ type: 'optional' }],
            description: 'Person or system assigned to the task',
          },
        ],
        validationRules: [],
        indexHints: ['status', 'priority', 'assignee'],
      },
    ],
    edgeTypes: [
      {
        id: 'depends_on',
        name: 'DependsOn',
        displayName: 'Depends On',
        description: 'Task dependency relationship',
        directionality: 'directed',
        allowedSourceTypes: ['Task'],
        allowedTargetTypes: ['Task'],
        cardinality: 'many-to-many',
        properties: [
          {
            name: 'dependencyType',
            type: 'String',
            constraints: [{ type: 'optional', enum: ['finish_to_start', 'start_to_start', 'finish_to_finish', 'start_to_finish'] }],
            description: 'Type of dependency',
          },
          {
            name: 'lag',
            type: 'Number',
            constraints: [{ type: 'optional', defaultValue: 0 }],
            description: 'Lag time in minutes',
          },
        ],
        weightConstraints: {
          required: false,
          min: 0,
          max: 1,
          defaultValue: 1,
        },
        validationRules: [],
      },
    ],
  },
  {
    id: 'organizational',
    name: 'Organizational Schema',
    description: 'Schema for organizational structures with people, departments, and reporting relationships',
    category: 'Organization',
    nodeTypes: [
      {
        id: 'person',
        name: 'Person',
        displayName: 'Person',
        description: 'Represents a person in the organization',
        icon: 'person',
        color: '#607d8b',
        properties: [
          {
            name: 'firstName',
            type: 'String',
            constraints: [{ type: 'required', maxLength: 50 }],
            description: 'First name',
          },
          {
            name: 'lastName',
            type: 'String',
            constraints: [{ type: 'required', maxLength: 50 }],
            description: 'Last name',
          },
          {
            name: 'email',
            type: 'String',
            constraints: [
              { type: 'required' },
              { type: 'unique' },
              { type: 'optional', pattern: '^[^@]+@[^@]+\\.[^@]+$' }
            ],
            description: 'Email address',
          },
          {
            name: 'jobTitle',
            type: 'String',
            constraints: [{ type: 'optional', maxLength: 100 }],
            description: 'Job title',
            examples: ['Software Engineer', 'Product Manager', 'CEO'],
          },
          {
            name: 'startDate',
            type: 'Date',
            constraints: [{ type: 'optional' }],
            description: 'Employment start date',
          },
        ],
        validationRules: [],
        indexHints: ['email', 'lastName', 'jobTitle'],
      },
      {
        id: 'department',
        name: 'Department',
        displayName: 'Department',
        description: 'Represents a department in the organization',
        icon: 'business',
        color: '#795548',
        properties: [
          {
            name: 'name',
            type: 'String',
            constraints: [{ type: 'required', maxLength: 100 }],
            description: 'Department name',
            examples: ['Engineering', 'Marketing', 'Human Resources'],
          },
          {
            name: 'budget',
            type: 'Number',
            constraints: [{ type: 'optional', minValue: 0 }],
            description: 'Department budget',
          },
          {
            name: 'location',
            type: 'String',
            constraints: [{ type: 'optional' }],
            description: 'Department location',
            examples: ['New York', 'San Francisco', 'Remote'],
          },
        ],
        validationRules: [],
        indexHints: ['name', 'location'],
      },
    ],
    edgeTypes: [
      {
        id: 'reports_to',
        name: 'ReportsTo',
        displayName: 'Reports To',
        description: 'Reporting relationship between people',
        directionality: 'directed',
        allowedSourceTypes: ['Person'],
        allowedTargetTypes: ['Person'],
        cardinality: 'many-to-many',
        properties: [
          {
            name: 'relationshipType',
            type: 'String',
            constraints: [{ type: 'optional', enum: ['direct', 'dotted_line', 'matrix'] }],
            description: 'Type of reporting relationship',
          },
        ],
        weightConstraints: {
          required: false,
          min: 0,
          max: 1,
          defaultValue: 1,
        },
        validationRules: [],
      },
      {
        id: 'member_of',
        name: 'MemberOf',
        displayName: 'Member Of',
        description: 'Person is a member of a department',
        directionality: 'directed',
        allowedSourceTypes: ['Person'],
        allowedTargetTypes: ['Department'],
        cardinality: 'many-to-many',
        properties: [
          {
            name: 'role',
            type: 'String',
            constraints: [{ type: 'optional' }],
            description: 'Role within the department',
            examples: ['Manager', 'Lead', 'Member'],
          },
          {
            name: 'joinDate',
            type: 'Date',
            constraints: [{ type: 'optional' }],
            description: 'Date joined the department',
          },
        ],
        weightConstraints: {
          required: false,
          min: 0,
          max: 1,
          defaultValue: 1,
        },
        validationRules: [],
      },
    ],
  },
];

export default schemaTemplates;