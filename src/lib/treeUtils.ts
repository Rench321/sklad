import { Node } from "../types";

/**
 * Find a node by ID in a tree structure
 */
export function findNodeById(nodes: Node[], id: string): Node | undefined {
    for (const node of nodes) {
        if (node.id === id) return node;
        if (node.children) {
            const found = findNodeById(node.children, id);
            if (found) return found;
        }
    }
    return undefined;
}

/**
 * Update a specific node in the tree by ID
 */
export function updateNodeInTree(
    nodes: Node[],
    id: string,
    updater: (node: Node) => Node
): Node[] {
    return nodes.map((node) => {
        if (node.id === id) return updater(node);
        if (node.children) {
            return { ...node, children: updateNodeInTree(node.children, id, updater) };
        }
        return node;
    });
}

/**
 * Remove a node from the tree by ID
 */
export function removeNodeFromTree(nodes: Node[], id: string): Node[] {
    return nodes
        .filter((node) => node.id !== id)
        .map((node) => {
            if (node.children) {
                return { ...node, children: removeNodeFromTree(node.children, id) };
            }
            return node;
        });
}

/**
 * Add a node to a parent (or root if parentId is null)
 */
export function addNodeToParent(
    nodes: Node[],
    parentId: string | null,
    newNode: Node
): Node[] {
    if (parentId === null) {
        return [...nodes, newNode];
    }

    return nodes.map((node) => {
        if (node.id === parentId) {
            return { ...node, children: [...(node.children || []), newNode] };
        }
        if (node.children) {
            return { ...node, children: addNodeToParent(node.children, parentId, newNode) };
        }
        return node;
    });
}

/**
 * Purge secret values from nodes (set to empty string)
 */
export function purgeSecretValues(nodes: Node[]): Node[] {
    return nodes.map((node) => ({
        ...node,
        value: node.isSecret ? "" : node.value,
        children: node.children ? purgeSecretValues(node.children) : undefined,
    }));
}

/**
 * Insert a node at a specific position relative to another node
 */
export function insertNodeAtPosition(
    nodes: Node[],
    nodeToInsert: Node,
    beforeNodeId: string | null | undefined
): Node[] {
    if (beforeNodeId === null || beforeNodeId === undefined) {
        return [...nodes, nodeToInsert];
    }

    const index = nodes.findIndex((n) => n.id === beforeNodeId);
    if (index === -1) {
        return [...nodes, nodeToInsert];
    }

    const result = [...nodes];
    result.splice(index, 0, nodeToInsert);
    return result;
}

/**
 * Check if a node is a descendant of another node
 */
export function isDescendantOf(
    nodes: Node[],
    nodeId: string,
    potentialAncestorId: string
): boolean {
    const findNode = (list: Node[]): Node | undefined => {
        for (const n of list) {
            if (n.id === nodeId) return n;
            if (n.children) {
                const found = findNode(n.children);
                if (found) return found;
            }
        }
        return undefined;
    };

    const checkDescendants = (node: Node): boolean => {
        if (node.id === potentialAncestorId) return true;
        if (node.children) {
            return node.children.some(checkDescendants);
        }
        return false;
    };

    const ancestor = findNode(nodes);
    return ancestor ? checkDescendants(ancestor) : false;
}
