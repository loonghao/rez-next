# SPDX-License-Identifier: Apache-2.0
# Copyright Contributors to the Rez Project

"""Utility modules for rez_next.

This module provides utility functions for the rez_next package,
including platform detection, file system operations, and string utilities.

Example usage::
    from rez_next.utils import get_platform_name, is_windows, which

    platform = get_platform_name()
    if is_windows():
        print("Running on Windows")
    path = which("python")
"""

from rez_next._native.util import (  # noqa: F401,F403
    get_rez_next_version,
    get_platform_name,
    get_architecture,
    get_platform_id,
    is_windows,
    is_linux,
    is_macos,
    is_unix,
    normalize_name,
    truncate_string,
    get_executable_name,
    which,
    which_all,
    # File system utilities
    expand_user_path,
    ensure_dir_exists,
    ensure_parent_dir_exists,
    is_writable,
    safe_remove,
    copy_file,
)

# Keep logging imports (temporarily disabled due to typo in filename)
# from rez_next.utils.loggging_ import (  # noqa: F401,F403
#     print_debug,
#     print_info,
#     print_warning,
#     print_error,
#     print_critical,
#     get_debug_printer,
#     get_info_printer,
#     get_warning_printer,
#     get_error_printer,
#     get_critical_printer,
#     log_duration,
# )
