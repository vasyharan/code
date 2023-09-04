# Buffers and Ropes

This document outlines the internals of Toku editor pane. It assumes prior knowledge of [ropes](https://en.wikipedia.org/wiki/Rope_(data_structure)) and [red-black trees](https://en.wikipedia.org/wiki/Red–black_tree). It is also advisable to pre-read parts 1 through 5 of the [rope science](https://xi-editor.io/docs/rope_science_00.html) articles from xi-editor docs.

## Buffer

Buffer represents the contents of an editor pane. It is resposible for managing the undo stack, as well as managing the memory of the editor contents.

A buffer contains a list of Ropes (this list serves as the undo stack of the buffer) and pointer to a Rope in the list for the *current* contents of the editor pane. When a buffer is created the list contains a single Rope &mdash; when creating a new buffer this Rope is empty, when opening a file this contains the contents of the file. Making an edit to the buffer appends a new Rope to the head of list of revisions (if the current revision pointer is not the head of the list, all revisions in front of the current revision pointer are removed), and moves the current revision pointer to the head. Undoing an edit moves the current revision pointer backwards through the list of revisions, redoing moves the pointer forwards.

A buffer manages the Blocks which contain the byte (text) contents of the buffer. There are two types of Blocks:
1. **MmapBlock** &ndash; are backed by an `mmap`-ed region. When opening an existing file; it's contents and `mmap`-ed and broken into fixed sixed blocks when creating the initial Rope. The contents of MmapBlocks are immutable.
2. **ByteArrayBlock** &ndash; are backed by append-only byte arrays. When inserts are performed the new bytes are appended to the block.

```dot process
digraph G {
    graph [fontname = "Virgil"];
    node [fontname = "Virgil",shape=circle];
    edge [fontname = "Virgil"];
    bgcolor=transparent;
    
    subgraph cluster_revisions { 
        label="revisions"; labeljust=l { 
        edge [dir=none];
        node [fixedsize=true];
        rank=same;
        rev0 [label="0"]; rev1 [label="1"]; revn1 [label="n-1"]; revn [label="n"]
        rev0 -> rev1;
        rev1 -> revn1 [style=dotted];
        revn1 -> revn;
    } }
    
    
    { # dotted black nodes
        node [shape=circle,style=dotted];
        n0 [group=0];
    }
    { # dotted red nodes
        node [shape=circle,color=red,style=dotted];
    }
    { # black nodes
        node [shape=circle];
        n1 [group=0]; n2 [group=0];
    }
    { # red nodes
        node [shape=circle,color=red];
        n4 [group=1]; n5 [group=1];
    }
    
    subgraph cluster_textblock { # text blocks
        label="byte array block"; labeljust=l labelloc=b { 
        node [shape="record"];
        block0 [label="<p0>H|<p1>e|<p2>l|<p3>l|<p4>o|<p5> |<p6>W|<p7>o|<p8>r|<p9>l|<p10>d|<p11>!|<p12>"];
    } }
    
    {
        node [label="",width=.1,style=invis];
        i0;
    }

    revn1 -> n0 [group=0];
    n0 -> i0 [style=invis];
    n0 -> n1 [group=0]; n0 -> n2 [group=0];
    
    
    revn -> n3 [group=1];
    n3 -> n4 [group=1]; n3 -> n5 [group=1];
    n4 -> n1 [group=1]; n4 -> n2  [group=1];
    n5 -> n6 [group=1]; n5 -> n7  [group=1];
    
    { rank=same; n0; n3; }
    { rank=same; n4; n5; }
    { rank=same; n1; n2; n6; n7; }
    
    n1 -> block0:p0; n1 -> block0:p3;
    n2 -> block0:p3; n2 -> block0:p6;
    n6 -> block0:p6; n6 -> block0:p9;
    n7 -> block0:p9; n7 -> block0:p12;
}
```
*Fig.1 &ndash; Overview of buffer revisions, ropes, and blocks.*

## Rope

Toku's Rope
Toku implements a [rope data structure](https://en.wikipedia.org/wiki/Rope_(data_structure)) representing the contents of an editor pane. This allows for efficient inserts and delete editor operations. The rope is implemented as a [red-black tree](https://en.wikipedia.org/wiki/Red–black_tree) to ensure it remains balanced. 

Toku's rope implementation consists of nodes and blocks. 

A node can be one of: 1) branch node, or 2) leaf node. Leaf nodes contain pointers to blocks (which contains the text) and metadata about the text (byte length, char length, etc.), and branch nodes contain the sum of the text metadata.

A block contains the text data as a byte array. There are two types of blocks: 1) byte array block, or 2) mmap block. A byte array block is an append only byte array, and a mmap block is a read-only mmap'ed block. mmap blocks are when a file is opened for edit (the initial contents are mmap'ed and made available for the rope). byte array blocks contains all inserts edits to the rope.
