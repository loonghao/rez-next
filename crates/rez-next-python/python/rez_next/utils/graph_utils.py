"""
Functions for manipulating dot-based resolve graphs.

Mirrors ``rez.utils.graph_utils`` API:
- ``read_graph_from_string`` — read graph from dot or compacted format string
- ``write_compacted`` — write graph in compacted string format
- ``write_dot`` — write graph in dot format (fast replacement for pygraph)
- ``prune_graph`` — prune graph to nodes accessible from a given package
- ``save_graph`` — render graph to an image file
- ``save_graph_object`` — render a pydot Dot object to an image file
- ``view_graph`` — view graph in an image viewer/browser

Uses a builtin ``Digraph`` class instead of ``rez.vendor.pygraph``.
Optional pydot support for image rendering.
"""
from __future__ import annotations

import os.path
import sys
import tempfile
from ast import literal_eval
from typing import Any, Iterable

from rez_next.utils.execution import Popen


# ── Builtin Digraph ──────────────────────────────────────────────────────────


class Digraph:
    """Simple directed graph with labeled edges and node/edge attributes.

    Mirrors a subset of ``pygraph.classes.digraph.digraph`` sufficient for
    Rez dependency graphs.
    """

    def __init__(self) -> None:
        # node_name -> list of (attr_key, attr_value)
        self.node_attr: dict[str, list[tuple[str, str]]] = {}
        # (from_name, to_name) -> list of (attr_key, attr_value)
        self.edge_attr: dict[tuple[str, str], list[tuple[str, str]]] = {}
        # (from_name, to_name) -> label string
        self.edge_label: dict[tuple[str, str], str] = {}
        # adjacency representation: from_name -> set of to_names
        self._edges_out: dict[str, set[str]] = {}
        # reverse adjacency: to_name -> set of from_names
        self._edges_in: dict[str, set[str]] = {}

    # -- node operations --

    def add_node(self, node: str, attrs: list[tuple[str, str]] | None = None) -> None:
        if node not in self.node_attr:
            self.node_attr[node] = []
            self._edges_out[node] = set()
            self._edges_in[node] = set()
        if attrs:
            for k, v in attrs:
                self._set_node_attr(node, k, v)

    def add_edge(
        self,
        edge: tuple[str, str],
        label: str = "",
        attrs: list[tuple[str, str]] | None = None,
    ) -> None:
        from_node, to_node = edge
        # ensure nodes exist
        self.add_node(from_node)
        self.add_node(to_node)
        self._edges_out[from_node].add(to_node)
        self._edges_in[to_node].add(from_node)
        if edge not in self.edge_attr:
            self.edge_attr[edge] = []
            self.edge_label[edge] = ""
        if attrs:
            for k, v in attrs:
                self._set_edge_attr(edge, k, v)
        if label:
            self.edge_label[edge] = label

    def has_node(self, node: str) -> bool:
        return node in self.node_attr

    def has_edge(self, edge: tuple[str, str]) -> bool:
        from_node, to_node = edge
        return from_node in self._edges_out and to_node in self._edges_out[from_node]

    def nodes(self) -> list[str]:
        return sorted(self.node_attr.keys())

    def edges(self) -> list[tuple[str, str]]:
        result = set()
        for from_node, to_nodes in self._edges_out.items():
            for to_node in to_nodes:
                result.add((from_node, to_node))
        return sorted(result)

    def neighbors(self, node: str) -> list[str]:
        """Return outgoing neighbors of *node*."""
        return sorted(self._edges_out.get(node, set()))

    def incidences(self, node: str) -> list[str]:
        """Return incoming neighbors of *node*."""
        return sorted(self._edges_in.get(node, set()))

    def node_attributes(self, node: str) -> list[tuple[str, str]]:
        return self.node_attr.get(node, [])

    def edge_attributes(self, edge: tuple[str, str]) -> list[tuple[str, str]]:
        return self.edge_attr.get(edge, [])

    def del_node(self, node: str) -> None:
        if node not in self.node_attr:
            return
        # remove all edges involving this node
        for from_node in list(self._edges_in.get(node, set())):
            self.del_edge((from_node, node))
        for to_node in list(self._edges_out.get(node, set())):
            self.del_edge((node, to_node))
        del self.node_attr[node]
        self._edges_out.pop(node, None)
        self._edges_in.pop(node, None)

    def del_edge(self, edge: tuple[str, str]) -> None:
        from_node, to_node = edge
        if from_node in self._edges_out:
            self._edges_out[from_node].discard(to_node)
        if to_node in self._edges_in:
            self._edges_in[to_node].discard(from_node)
        self.edge_attr.pop(edge, None)
        self.edge_label.pop(edge, None)

    def reverse(self) -> Digraph:
        """Return a new graph with reversed edge directions."""
        g = Digraph()
        for node in self.nodes():
            g.add_node(node, attrs=self.node_attributes(node)[:])
        for edge in self.edges():
            from_node, to_node = edge
            g.add_edge(
                (to_node, from_node),
                label=self.edge_label.get(edge, ""),
                attrs=self.edge_attributes(edge)[:],
            )
        return g

    def _set_node_attr(self, node: str, key: str, value: str) -> None:
        existing = self.node_attr.setdefault(node, [])
        for i, (k, v) in enumerate(existing):
            if k == key:
                existing[i] = (key, value)
                return
        existing.append((key, value))

    def _set_edge_attr(self, edge: tuple[str, str], key: str, value: str) -> None:
        existing = self.edge_attr.setdefault(edge, [])
        for i, (k, v) in enumerate(existing):
            if k == key:
                existing[i] = (key, value)
                return
        existing.append((key, value))


# ── Dot format read/write (replacing rez.vendor.pygraph.readwrite.dot) ──────


def _parse_dot(txt: str) -> Digraph:
    """Parse a simple DOT-format string into a ``Digraph``.

    This is a minimalist parser sufficient for Rez's generated graphs.
    It does NOT support the full DOT language.
    """
    g = Digraph()

    lines = txt.splitlines()
    lines = [l.strip() for l in lines if l.strip()]

    # skip the opening "digraph g {" line
    in_graph = False
    for line in lines:
        if line.startswith("digraph") and "{" in line:
            in_graph = True
            continue
        if not in_graph:
            continue
        if line == "}":
            break

        # remove trailing ;
        line = line.rstrip(";").strip()

        if "->" in line:
            # edge: node -> node [attrs];
            parts = line.split("->")
            from_node = parts[0].strip()
            rest = parts[1].strip()
            if "[" in rest:
                to_node_part = rest[: rest.index("[")].strip()
                attrs_part = rest[rest.index("[") + 1: rest.rindex("]")]
            else:
                to_node_part = rest
                attrs_part = ""
            to_node = to_node_part.strip()

            attrs = _parse_dot_attrs(attrs_part)
            label = ""
            for k, v in attrs:
                if k == "label":
                    label = v.strip('"')
                    break

            g.add_edge((from_node, to_node), label=label, attrs=attrs)
        else:
            # node: node [attrs];
            if "[" in line:
                node_name = line[: line.index("[")].strip()
                attrs_part = line[line.index("[") + 1: line.rindex("]")]
            else:
                node_name = line.strip()
                attrs_part = ""
            attrs = _parse_dot_attrs(attrs_part)
            g.add_node(node_name, attrs=attrs)

    return g


def _parse_dot_attrs(attrs_str: str) -> list[tuple[str, str]]:
    """Parse ``key="value", key2="value2"`` from DOT bracket content."""
    if not attrs_str.strip():
        return []
    result: list[tuple[str, str]] = []
    # split on commas but respect quoted strings
    parts = _split_dot_attrs(attrs_str)
    for part in parts:
        part = part.strip()
        if "=" in part:
            k, v = part.split("=", 1)
            k = k.strip()
            v = v.strip().strip('"').strip("'")
            result.append((k, v))
    return result


def _split_dot_attrs(s: str) -> list[str]:
    """Split comma-separated DOT attributes, respecting quotes."""
    parts: list[str] = []
    current: list[str] = []
    in_quote = False
    quote_char: str | None = None
    for ch in s:
        if ch in ('"', "'"):
            if in_quote and ch == quote_char:
                in_quote = False
                quote_char = None
            elif not in_quote:
                in_quote = True
                quote_char = ch
            current.append(ch)
        elif ch == "," and not in_quote:
            parts.append("".join(current))
            current = []
        else:
            current.append(ch)
    if current:
        parts.append("".join(current))
    return parts


def _accessibility(g: Digraph) -> dict[str, set[str]]:
    """Compute the accessibility (transitive closure) of a digraph.

    Returns a dict mapping each node to the set of nodes reachable from it
    (including the node itself).
    """
    # Floyd-Warshall-like or BFS from each node
    nodes = g.nodes()
    result: dict[str, set[str]] = {}

    def _reachable(start: str) -> set[str]:
        visited: set[str] = set()
        stack = [start]
        while stack:
            node = stack.pop()
            if node in visited:
                continue
            visited.add(node)
            for nb in g.neighbors(node):
                if nb not in visited:
                    stack.append(nb)
        return visited

    for node in nodes:
        result[node] = _reachable(node)

    return result


# ── Main API (mirrors rez.utils.graph_utils) ────────────────────────────────


def read_graph_from_string(txt: str) -> Digraph:
    """Read a graph from a string, either in dot format, or our own compacted format.

    Args:
        txt: Dot-format string or compacted format string.

    Returns:
        ``Digraph`` object.
    """
    if not txt.startswith("{"):
        return _parse_dot(txt)

    def conv(value: Any) -> str:
        if isinstance(value, str):
            return '"' + value + '"'
        return str(value)

    doc = literal_eval(txt)
    g = Digraph()

    for attrs, values in doc.get("nodes", []):
        attrs_list: list[tuple[str, str]] = [(k, conv(v)) for k, v in attrs]
        for value in values:
            if isinstance(value, str):
                node_name = value
                node_attrs = attrs_list
            else:
                node_name, label = value
                node_attrs = attrs_list + [("label", conv(label))]
            g.add_node(node_name, attrs=node_attrs)

    for attrs, values in doc.get("edges", []):
        attrs_list = [(k, conv(v)) for k, v in attrs]
        for value in values:
            if len(value) == 3:
                edge = value[:2]
                label = value[-1]
            else:
                edge = value
                label = ""
            g.add_edge(edge, label=label, attrs=attrs_list)

    return g


def write_compacted(g: Digraph) -> str:
    """Write a graph in our own compacted format.

    Returns:
        Compacted string representation.
    """
    d_nodes: dict[tuple, list] = {}
    d_edges: dict[tuple, list] = {}

    def conv(value: Any) -> str:
        if isinstance(value, str):
            return value.strip('"')
        return str(value)

    for node in g.nodes():
        label: str | None = None
        attrs: list[tuple[str, str]] = []
        for k, v in g.node_attributes(node):
            v_ = conv(v)
            if k == "label":
                label = v_
            else:
                attrs.append((k, v_))

        value: str | tuple[str, str] = (node, label) if label else node
        d_nodes.setdefault(tuple(attrs), []).append(value)

    for edge in g.edges():
        edge_attrs = [(k, conv(v)) for k, v in sorted(g.edge_attributes(edge))]
        label = str(g.edge_label.get(edge, ""))
        if label:
            value = tuple(list(edge) + [label])
        else:
            value = edge
        d_edges.setdefault(tuple(edge_attrs), []).append(tuple(value))

    doc = dict(nodes=list(d_nodes.items()), edges=list(d_edges.items()))
    return str(doc)


def write_dot(g: Digraph) -> str:
    """Write a graph in dot format.

    This is a faster replacement for ``pygraph.readwrite.dot.write``,
    sufficient for Rez-generated graphs.

    Args:
        g: Input graph.

    Returns:
        Graph in dot format.
    """
    lines = ["digraph g {"]

    def attrs_txt(items: list[tuple[str, str]]) -> str:
        if items:
            txt = ", ".join(
                '%s="%s"' % (k, str(v).strip('"'))
                for k, v in items
            )
            return "[" + txt + "]"
        return ""

    for node in g.nodes():
        atxt = attrs_txt(g.node_attributes(node))
        txt = "%s %s;" % (node, atxt)
        lines.append(txt)

    for e in g.edges():
        edge_from, edge_to = e
        attrs = g.edge_attributes(e)

        label = str(g.edge_label.get(e, ""))
        if label:
            attrs.append(("label", label))

        atxt = attrs_txt(attrs)
        txt = "%s -> %s %s;" % (edge_from, edge_to, atxt)
        lines.append(txt)

    lines.append("}")
    return "\n".join(lines)


def prune_graph(graph_str: str, package_name: str) -> str:
    """Prune a package graph so it only contains nodes accessible from the
    given package.

    Args:
        graph_str: Dot-language graph string.
        package_name: Name of package of interest.

    Returns:
        Pruned graph, as a dot string.
    """
    import rez_next._native as _native

    g = _parse_dot(graph_str)
    nodes = set()

    for node, attrs in g.node_attr.items():
        label_attrs = [x for x in attrs if x[0] == "label"]
        if label_attrs:
            label = label_attrs[0][1]
            try:
                req_str = _request_from_label(label)
                request = _native.PackageRequirement(req_str)
            except Exception:
                continue

            if request.name == package_name:
                nodes.add(node)

    if not nodes:
        raise ValueError(
            "The package %r does not appear in the graph." % package_name
        )

    # find nodes upstream from these nodes
    g_rev = g.reverse()
    accessible_nodes: set[str] = set()
    access = _accessibility(g_rev)
    for node in nodes:
        nodes_ = access.get(node, set())
        accessible_nodes |= nodes_

    # remove inaccessible nodes
    inaccessible_nodes = set(g.nodes()) - accessible_nodes
    for node in inaccessible_nodes:
        g.del_node(node)

    return write_dot(g)


def save_graph(
    graph_str: str,
    dest_file: str,
    fmt: str | None = None,
    image_ratio: float | None = None,
) -> str:
    """Render a graph to an image file.

    Args:
        graph_str: Dot-language graph string.
        dest_file: Filepath to save to.
        fmt: Format (e.g. ``"png"``, ``"jpg"``).  Inferred from extension if
            not provided.
        image_ratio: Optional image ratio.

    Returns:
        String representing the format written (e.g. ``'png'``).

    Raises:
        RuntimeError: If no graph or multiple graphs are generated.
    """
    try:
        import pydot
    except ImportError:
        raise ImportError(
            "pydot is required to render graphs. "
            "Install it with: pip install pydot"
        )

    graphs = pydot.graph_from_dot_data(graph_str)

    if not graphs:
        raise RuntimeError("No graph generated")

    if len(graphs) > 1:
        path, ext = os.path.splitext(dest_file)
        dest_files = []

        for i, g_i in enumerate(graphs):
            try:
                dest_file_ = "%s.%d%s" % (path, i + 1, ext)
                save_graph_object(g_i, dest_file_, fmt, image_ratio)
                dest_files.append(dest_file_)
            except Exception:
                pass

        raise RuntimeError(
            "More than one graph was generated; this probably indicates a bug "
            "in graph generation. Graphs were written to %r" % dest_files
        )

    return save_graph_object(graphs[0], dest_file, fmt, image_ratio)


def save_graph_object(
    g: Any,
    dest_file: str,
    fmt: str | None = None,
    image_ratio: float | None = None,
) -> str:
    """Like ``save_graph``, but takes a pydot Dot object directly.

    Args:
        g: A ``pydot.Dot`` object.
        dest_file: Filepath to save to.
        fmt: Format (defaults to extension or ``"png"``).
        image_ratio: Optional image ratio.

    Returns:
        String representing the format written.
    """
    if fmt is None:
        fmt = os.path.splitext(dest_file)[1].lower().strip(".") or "png"

    write_fn_name = "write_" + fmt
    if not hasattr(g, write_fn_name):
        raise RuntimeError("Unsupported graph format: '%s'" % fmt)

    if image_ratio is not None:
        g.set_ratio(str(image_ratio))

    write_fn = getattr(g, write_fn_name)
    write_fn(dest_file)
    return fmt


def view_graph(graph_str: str, dest_file: str | None = None) -> None:
    """View a dot graph in an image viewer.

    Args:
        graph_str: Dot-language graph string.
        dest_file: Optional destination file.  A temp file is used if not
            provided.
    """
    from rez_next.config import config
    from rez_next.system import system

    if (system.platform == "linux") and (not os.getenv("DISPLAY")):
        print("Unable to open display.", file=sys.stderr)
        sys.exit(1)

    dest_file = _write_graph(graph_str, dest_file=dest_file)

    viewed = False
    prog = config.image_viewer or "browser"
    print("loading image viewer (%s)..." % prog)

    if config.image_viewer:
        with _optional_popen([config.image_viewer, dest_file]) as p:
            p.wait()
            viewed = not bool(p.returncode)

    if not viewed:
        import webbrowser

        webbrowser.open_new("file://" + dest_file)


# ── Helpers (internal) ──────────────────────────────────────────────────────


def _request_from_label(label: str) -> str:
    """Convert a DOT label like ``'"PyQt-4.8.0[1]"'`` to ``'PyQt-4.8.0'``."""
    return label.strip('"').strip("'").rsplit("[", 1)[0]


def _write_graph(graph_str: str, dest_file: str | None = None) -> str:
    """Render graph to a temp file or given path and return the path."""
    from rez_next.config import config

    if not dest_file:
        tmpf = tempfile.mkstemp(
            prefix="resolve-dot-",
            suffix="." + config.dot_image_format,
        )
        os.close(tmpf[0])
        dest_file = tmpf[1]

    print("rendering image to " + dest_file + "...")
    save_graph(graph_str, dest_file)
    return dest_file


def _optional_popen(args: list[str], **kwargs) -> Any:
    """Context manager wrapper around Popen."""
    return Popen(args, **kwargs)
