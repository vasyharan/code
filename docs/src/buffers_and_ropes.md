# Buffers and Ropes

```dot process
digraph G {
    graph [fontname = "Virgil"];
    node [fontname = "Virgil",shape=circle];
    // node [shape=circle];
    edge [fontname = "Virgil"];

    bgcolor=transparent;
    
    subgraph cluster_revisions { 
        label="revisions"; labeljust=l { 
        rank=same;
        edge [dir=none];
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
        label="text block"; labeljust=l labelloc=b { 
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
