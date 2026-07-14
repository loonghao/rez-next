"""Tests for rez_next.utils.graph_utils module."""
from __future__ import annotations

from rez_next.utils.graph_utils import (
    Digraph,
    read_graph_from_string,
    write_dot,
    write_compacted,
    prune_graph,
    _accessibility,
)
from rez_next.utils import graph_utils


def test_digraph_basic():
    g = Digraph()
    g.add_node("a", [("label", "python-3.9")])
    g.add_node("b", [("label", "maya-2024")])
    g.add_edge(("a", "b"), label="depends")
    assert g.has_node("a")
    assert g.has_node("b")
    assert g.has_edge(("a", "b"))
    assert g.nodes() == ["a", "b"]
    assert g.edges() == [("a", "b")]
    assert g.neighbors("a") == ["b"]
    assert g.incidences("b") == ["a"]


def test_digraph_reverse():
    g = Digraph()
    g.add_edge(("a", "b"))
    g.add_edge(("b", "c"))
    rev = g.reverse()
    assert rev.has_edge(("b", "a"))
    assert rev.has_edge(("c", "b"))


def test_digraph_del_node():
    g = Digraph()
    g.add_edge(("a", "b"))
    g.add_edge(("b", "c"))
    g.del_node("b")
    assert not g.has_node("b")
    assert g.nodes() == ["a", "c"]


def test_write_dot():
    g = Digraph()
    g.add_node("a", [("label", "python-3.9")])
    g.add_node("b", [("label", "maya-2024")])
    g.add_edge(("a", "b"), label="depends")
    dot = write_dot(g)
    assert dot.startswith("digraph g {")
    assert "a -> b" in dot
    assert "label" in dot


def test_read_dot():
    dot = """digraph g {
a [label="python-3.9"];
b [label="maya-2024"];
a -> b [label="depends"];
}"""
    g = read_graph_from_string(dot)
    assert g.nodes() == ["a", "b"]
    assert g.edges() == [("a", "b")]
    assert g.node_attributes("a") == [("label", "python-3.9")]


def test_write_compacted():
    g = Digraph()
    g.add_node("a", [("label", "python-3.9")])
    g.add_node("b")
    g.add_edge(("a", "b"), label="depends")
    compact = write_compacted(g)
    assert isinstance(compact, str)
    # round-trip
    g2 = read_graph_from_string(compact)
    assert g2.nodes() == ["a", "b"]


def test_read_compacted_format():
    """Test reading the compacted format directly."""
    compact = "{'nodes': [((), ['a', ('b', 'python-3.9')])], 'edges': [((), [('a', 'b', 'depends')])]}"
    g = read_graph_from_string(compact)
    assert g.nodes() == ["a", "b"]
    assert g.edges() == [("a", "b")]


def test_prune_graph():
    dot = """digraph g {
a [label="python-3.9"];
b [label="maya-2024"];
c [label="nuke-13"];
a -> b [label="depends"];
a -> c [label="depends"];
}"""
    pruned = prune_graph(dot, "maya")
    assert "a" in pruned
    assert "b" in pruned
    assert "c" not in pruned


def test_accessibility():
    g = Digraph()
    g.add_edge(("a", "b"))
    g.add_edge(("b", "c"))
    g.add_edge(("a", "d"))
    access = _accessibility(g)
    assert "b" in access.get("a", set())
    assert "c" in access.get("a", set())
    assert "d" in access.get("a", set())
    assert "c" in access.get("b", set())
    assert "a" in access.get("a", set())


def test_utils_import():
    """Verify graph_utils is accessible via rez_next.utils.graph_utils."""
    assert hasattr(graph_utils, "Digraph")
    assert hasattr(graph_utils, "read_graph_from_string")
    assert hasattr(graph_utils, "write_dot")
    assert hasattr(graph_utils, "write_compacted")
    assert hasattr(graph_utils, "prune_graph")
    assert hasattr(graph_utils, "save_graph")
    assert hasattr(graph_utils, "save_graph_object")
    assert hasattr(graph_utils, "view_graph")
