"""
Python source code wrapper and decorators for Rez-next.

Mirrors ``rez.utils.sourcecode`` — provides ``SourceCode``, ``late()``,
``early()``, ``include()`` decorators, and ``IncludeModuleManager``.
"""
from __future__ import annotations

import os.path
import traceback
from glob import glob
from inspect import getsourcelines
from textwrap import dedent
from types import CodeType, ModuleType
from typing import Callable, Generic, TypeVar, TYPE_CHECKING

if TYPE_CHECKING:
    from typing import Any

from rez_next.utils.data_utils import cached_property
from rez_next.utils.formatting import indent
from rez_next.util import load_module_from_file

T = TypeVar("T")
CallabeT = TypeVar("CallabeT", bound=Callable)


def early() -> Callable[[CallabeT], CallabeT]:
    """Used by functions in package.py to harden to the return value at build
    time.

    The term 'early' refers to the fact these package attributes are evaluated
    early, ie at build time and before a package is installed.

    Rez API: ``rez.utils.sourcecode.early()``
    """
    def decorated(fn: CallabeT) -> CallabeT:
        setattr(fn, "_early", True)
        return fn
    return decorated


def late() -> Callable[[CallabeT], CallabeT]:
    """Used by functions in package.py that are evaluated lazily.

    The term 'late' refers to the fact these package attributes are evaluated
    late, ie when the attribute is queried for the first time.

    If you want to implement a package.py attribute as a function, you MUST use
    this decorator - otherwise it is understood that you want your attribute to
    be a function, not the return value of that function.

    Rez API: ``rez.utils.sourcecode.late()``
    """
    from rez_next.package_resources import package_rex_keys

    def decorated(fn: CallabeT) -> CallabeT:
        if fn.__name__ in package_rex_keys:
            raise ValueError(
                "Cannot use @late decorator on function '%s'" % fn.__name__
            )
        setattr(fn, "_late", True)
        _add_decorator(fn, "late")
        return fn
    return decorated


def include(module_name: str, *module_names: str) -> Callable[[CallabeT], CallabeT]:
    """Used by functions in package.py to have access to named modules.

    See the ``package_definition_python_path`` config setting for more info.

    Rez API: ``rez.utils.sourcecode.include()``
    """
    def decorated(fn: CallabeT) -> CallabeT:
        _add_decorator(fn, "include", nargs=[module_name] + list(module_names))
        return fn
    return decorated


def _add_decorator(fn, name: str, **kwargs) -> None:
    if not hasattr(fn, "_decorators"):
        setattr(fn, "_decorators", [])
    kwargs.update({"name": name})
    fn._decorators.append(kwargs)


class SourceCodeError(Exception):
    """Base exception for source code errors."""
    def __init__(self, msg: str, short_msg: str) -> None:
        super().__init__(msg)
        self.short_msg = short_msg


class SourceCodeCompileError(SourceCodeError):
    """Raised when source code fails to compile."""
    pass


class SourceCodeExecError(SourceCodeError):
    """Raised when source code fails to execute."""
    pass


class SourceCode(Generic[T]):
    """Wrapper for python source code.

    This object is aware of the decorators defined in this sourcefile (such as
    ``include``) and deals with them appropriately.

    Rez API: ``rez.utils.sourcecode.SourceCode``
    """
    def __init__(
        self,
        source: str | None = None,
        func: Callable[..., T] | None = None,
        filepath: str | None = None,
        eval_as_function: bool = True,
    ) -> None:
        self.source = (source or '').rstrip()
        self.func = func
        self.filepath = filepath
        self.eval_as_function = eval_as_function
        self.package = None  # type: Any | None

        self.funcname: str | None = None
        self.decorators: list[dict] = []

        if self.func is not None:
            self._init_from_func()

    def copy(self) -> SourceCode[T]:
        other = SourceCode.__new__(SourceCode)
        other.source = self.source
        other.func = self.func
        other.filepath = self.filepath
        other.eval_as_function = self.eval_as_function
        other.package = self.package
        other.funcname = self.funcname
        other.decorators = self.decorators
        return other

    def _init_from_func(self) -> None:
        self.funcname = self.func.__name__
        self.decorators = getattr(self.func, "_decorators", [])

        loc = getsourcelines(self.func)[0][len(self.decorators) + 1:]
        code = dedent(''.join(loc))

        codelines = code.split('\n')
        linescount = len(codelines)

        for i, line in enumerate(codelines):
            if line.startswith('#'):
                nextindex = i + 1 if i < linescount else i - 1
                nextline = codelines[nextindex]
                while nextline.startswith('#'):
                    nextline = codelines[nextindex]
                    nextindex = (
                        nextindex + 1 if nextindex < linescount
                        else nextindex - 1
                    )
                firstchar = len(nextline) - len(nextline.lstrip())
                codelines[i] = '%s%s' % (nextline[:firstchar], line)

        code = '\n'.join(codelines).rstrip()
        code = dedent(code)
        self.source = code

    @cached_property
    def includes(self) -> set | None:
        info = self._get_decorator_info("include")
        if not info:
            return None
        return set(info.get("nargs", []))

    @cached_property
    def late_binding(self) -> bool:
        info = self._get_decorator_info("late")
        return bool(info)

    @cached_property
    def evaluated_code(self) -> str:
        if self.eval_as_function:
            funcname = self.funcname or "_unnamed"
            code = indent(self.source)
            code = (
                "def %s():\n" % funcname
                + code
                + "\n_result = %s()" % funcname
            )
        else:
            code = "if True:\n" + indent(self.source)
        return code

    @property
    def sourcename(self) -> str:
        if self.filepath:
            filename = self.filepath
        else:
            filename = "string"
        if self.funcname:
            filename += ":%s" % self.funcname
        return "<%s>" % filename

    @cached_property
    def compiled(self) -> CodeType:
        try:
            pyc = compile(self.evaluated_code, self.sourcename, 'exec')
        except Exception as e:
            stack = traceback.format_exc()
            raise SourceCodeCompileError(
                "Failed to compile %s:\n%s" % (self.sourcename, stack),
                short_msg=str(e),
            )
        return pyc

    def exec_(self, globals_: dict | None = None) -> T:
        """Execute the source code and return the result."""
        if globals_ is None:
            globals_ = {}
        if self.package is not None and self.includes:
            for name in self.includes:
                module = include_module_manager.load_module(name, self.package)
                globals_[name] = module

        pyc = self.compiled
        try:
            exec(pyc, globals_)
        except Exception as e:
            stack = traceback.format_exc()
            raise SourceCodeExecError(
                "Failed to execute %s:\n%s" % (self.sourcename, stack),
                short_msg=str(e),
            )
        return globals_.get("_result")

    def to_text(self, funcname: str) -> str:
        """Return source as text with decorator annotations."""
        if self.source[0] in (' ', '\t'):
            source = self.source
        else:
            source = indent(self.source)
        txt = "def %s():\n%s" % (funcname, source)
        for entry in self.decorators:
            nargs_str = ", ".join(map(repr, entry.get("nargs", [])))
            name_str = entry.get("name")
            sig = "@%s(%s)" % (name_str, nargs_str)
            txt = sig + '\n' + txt
        return txt

    def _get_decorator_info(self, name: str) -> dict | None:
        matches = [x for x in self.decorators if x.get("name") == name]
        if not matches:
            return None
        return matches[0]

    def __getstate__(self) -> dict:
        return {
            "source": self.source,
            "filepath": self.filepath,
            "funcname": self.funcname,
            "eval_as_function": self.eval_as_function,
            "decorators": self.decorators,
        }

    def __setstate__(self, state: dict) -> None:
        self.source = state["source"]
        self.filepath = state["filepath"]
        self.funcname = state["funcname"]
        self.eval_as_function = state["eval_as_function"]
        self.decorators = state["decorators"]
        self.func = None
        self.package = None

    def __eq__(self, other: object) -> bool:
        return isinstance(other, SourceCode) and other.source == self.source

    def __ne__(self, other: object) -> bool:
        return not (other == self)

    def __str__(self) -> str:
        return self.source

    def __repr__(self) -> str:
        return "%s(%r)" % (self.__class__.__name__, self.source)


class IncludeModuleManager:
    """Manages a cache of modules imported via ``@include`` decorator.

    Rez API: ``rez.utils.sourcecode.IncludeModuleManager``
    """
    include_modules_subpath = ".rez/include"

    def __init__(self) -> None:
        self.modules: dict[str, ModuleType] = {}

    def load_module(
        self,
        name: str,
        package: Any,
    ) -> ModuleType | None:
        from hashlib import sha1
        from rez_next.config import config
        from rez_next.developer_package import DeveloperPackage

        if isinstance(package, DeveloperPackage):
            path = config.package_definition_python_path
            filepath = os.path.join(path, "%s.py" % name)
            if not os.path.exists(filepath):
                return None
            with open(filepath, "rb") as f:
                hash_str = sha1(f.read().strip()).hexdigest()
        else:
            path = os.path.join(package.base, self.include_modules_subpath)
            pathname = os.path.join(path, "%s.py" % name)
            hashname = os.path.join(path, "%s.sha1" % name)

            if os.path.isfile(pathname) and os.path.isfile(hashname):
                with open(hashname, "r") as f:
                    hash_str = f.readline()
                filepath = pathname
            else:
                pathname = os.path.join(path, "%s-*.py" % name)
                hashnames = glob(pathname)
                if not hashnames:
                    return None
                filepath = hashnames[0]
                hash_str = filepath.rsplit('-', 1)[-1].split('.', 1)[0]

        module = self.modules.get(hash_str)
        if module is not None:
            return module

        if config.debug and config.debug("file_loads"):
            from rez_next.utils.logging_ import print_debug
            print_debug("Loading include sourcefile: %s" % filepath)

        module = load_module_from_file(name, filepath)
        self.modules[hash_str] = module
        return module


# singleton
include_module_manager = IncludeModuleManager()
