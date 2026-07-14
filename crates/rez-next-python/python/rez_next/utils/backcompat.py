"""
Utility code for supporting earlier Rez data in later Rez releases.

Mirrors ``rez.utils.backcompat``.
"""
from __future__ import annotations

import os
import os.path
import re
import textwrap


variant_key_conversions = {
    "name": "name",
    "version": "version",
    "index": "index",
    "search_path": "location",
}


def convert_old_variant_handle(handle_dict: dict) -> dict:
    """Convert a variant handle from serialize_version < 4.0.

    Args:
        handle_dict: Old variant handle dict.

    Returns:
        Converted dict with ``key`` and ``variables`` fields.
    """
    old_variables = handle_dict.get("variables", {})
    variables = dict(repository_type="filesystem")

    for old_key, key in variant_key_conversions.items():
        value = old_variables.get(old_key)
        variables[key] = value

    path = handle_dict["path"]
    filename = os.path.basename(path)
    if os.path.splitext(filename)[0] == "package":
        key = "filesystem.variant"
    else:
        key = "filesystem.variant.combined"

    return dict(key=key, variables=variables)


def convert_old_command_expansions(command: str) -> str:
    """Convert expansions from ``!OLD!`` style to ``{new}``.

    Rez API: ``rez.utils.backcompat.convert_old_command_expansions()``
    """
    command = command.replace("!VERSION!", "{version}")
    command = command.replace("!MAJOR_VERSION!", "{version.major}")
    command = command.replace("!MINOR_VERSION!", "{version.minor}")
    command = command.replace("!BASE!", "{base}")
    command = command.replace("!ROOT!", "{root}")
    command = command.replace("!USER!", "{system.user}")
    return command


within_unescaped_quotes_regex = re.compile('(?<!\\\\)"(.*?)(?<!\\\\)"')

_debug_printer = None


def _get_debug_printer():
    global _debug_printer
    if _debug_printer is None:
        from rez_next.utils.logging_ import get_debug_printer
        _debug_printer = get_debug_printer()
    return _debug_printer


def convert_old_commands(commands: list[str], annotate: bool = True) -> str:
    """Converts old-style package commands into equivalent Rex code.

    Args:
        commands: List of old-style command strings.
        annotate: If True, annotate the generated Rex code with the original
            commands as comments.

    Returns:
        Rex code as a string.

    Rez API: ``rez.utils.backcompat.convert_old_commands()``
    """
    from rez_next.config import config

    def _repl(s: str) -> str:
        return s.replace('\\"', '"')

    def _encode(s: str) -> str:
        s_new = ''
        prev_i = 0
        for m in within_unescaped_quotes_regex.finditer(s):
            s_ = s[prev_i:m.start()]
            s_new += _repl(s_)
            s_new += s[m.start():m.end()]
            prev_i = m.end()
        s_ = s[prev_i:]
        s_new += _repl(s_)
        return repr(s_new)

    loc: list[str] = []

    for cmd in commands:
        if annotate:
            txt = "OLD COMMAND: %s" % cmd
            line = "comment(%s)" % _encode(txt)
            loc.append(line)

        cmd = convert_old_command_expansions(cmd)
        toks = cmd.strip().split()

        try:
            if toks[0] == "export":
                var, value = cmd.split(' ', 1)[1].split('=', 1)
                for bookend in ('"', "'"):
                    if value.startswith(bookend) and value.endswith(bookend):
                        value = value[1:-1]
                        break

                separator = config.env_var_separators.get(var, ":")

                if var == "CMAKE_MODULE_PATH":
                    value = value.replace("'%s'" % separator, separator)
                    value = value.replace('"%s"' % separator, separator)
                    value = value.replace(":", separator)

                parts = value.split(separator)
                parts = [x for x in parts if x]
                if len(parts) > 1:
                    idx = None
                    var1 = "$%s" % var
                    var2 = "${%s}" % var
                    if var1 in parts:
                        idx = parts.index(var1)
                    elif var2 in parts:
                        idx = parts.index(var2)
                    if idx in (0, len(parts) - 1):
                        func = "appendenv" if idx == 0 else "prependenv"
                        parts = parts[1:] if idx == 0 else parts[:-1]
                        val = separator.join(parts)
                        loc.append("%s('%s', %s)" % (func, var, _encode(val)))
                        continue

                loc.append("setenv('%s', %s)" % (var, _encode(value)))

            elif toks[0].startswith('#'):
                loc.append("comment(%s)" % _encode(' '.join(toks[1:])))

            elif toks[0] == "alias":
                match = re.search(r"alias (?P<key>.*?)=(?P<value>.*)", cmd)
                key = match.groupdict()['key'].strip()
                value = match.groupdict()['value'].strip()
                if (value.startswith('"') and value.endswith('"')) or \
                        (value.startswith("'") and value.endswith("'")):
                    value = value[1:-1]
                loc.append("alias('%s', %s)" % (key, _encode(value)))

            else:
                loc.append("command(%s)" % _encode(cmd))

        except Exception:
            loc.append("command(%s)" % _encode(cmd))

    rex_code = '\n'.join(loc)
    if config.debug("old_commands"):
        br = '-' * 80
        msg = textwrap.dedent(
            """
            %s
            OLD COMMANDS:
            %s

            NEW COMMANDS:
            %s
            %s
            """) % (br, '\n'.join(commands), rex_code, br)
        _get_debug_printer()(msg)
    return rex_code
