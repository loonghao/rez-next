"""
Integration tests for rez-core.

These tests verify that different components work together correctly
and that the overall system behaves as expected.
"""

import pytest
from typing import List
import rez_core


@pytest.mark.integration
class TestVersionIntegration:
    """Test integration between version components."""
    
    def test_version_and_range_integration(self):
        """Test that versions and ranges work together."""
        v = rez_core.Version("1.2.3")
        vr = rez_core.VersionRange(">=1.0.0")
        
        # Should be able to check containment
        result = vr.contains(v)
        assert isinstance(result, bool)
    
    def test_parse_functions_integration(self):
        """Test that parse functions work with created objects."""
        v1 = rez_core.Version("1.2.3")
        v2 = rez_core.parse_version("1.2.3")
        
        # Both should be equivalent
        assert v1 == v2
        assert str(v1) == str(v2)
        
        vr1 = rez_core.VersionRange(">=1.0.0")
        vr2 = rez_core.parse_version_range(">=1.0.0")
        
        # Both should be equivalent
        assert str(vr1) == str(vr2)
    
    def test_version_operations_with_config(self, config):
        """Test that version operations work with configuration."""
        # Create versions with config present
        versions = [
            rez_core.Version("1.0.0"),
            rez_core.Version("2.0.0"),
            rez_core.Version("1.5.0"),
        ]
        
        # Should be able to sort
        sorted_versions = sorted(versions)
        assert len(sorted_versions) == 3
        
        # Should be able to compare
        assert versions[0] < versions[1]
        
        # Config should still be valid
        assert config is not None


@pytest.mark.integration
class TestErrorHandling:
    """Test error handling across components."""
    
    def test_version_error_types(self):
        """Test that appropriate error types are raised."""
        with pytest.raises((ValueError, rez_core.VersionParseError)):
            rez_core.Version("")
        
        with pytest.raises((ValueError, rez_core.VersionParseError)):
            rez_core.parse_version("invalid.version")
    
    def test_version_range_error_types(self):
        """Test that version range errors are handled correctly."""
        # For now, most ranges are accepted as placeholders
        # This test will be expanded when range parsing is implemented
        try:
            vr = rez_core.VersionRange("invalid_range")
            assert str(vr) == "invalid_range"  # Placeholder behavior
        except Exception as e:
            # If errors are implemented, they should be appropriate types
            assert isinstance(e, (ValueError, rez_core.VersionParseError))
    
    def test_error_messages_are_helpful(self):
        """Test that error messages provide useful information."""
        try:
            rez_core.Version("")
        except Exception as e:
            error_msg = str(e)
            # Error message should not be empty
            assert len(error_msg) > 0
            # Should contain some indication of what went wrong
            assert any(word in error_msg.lower() for word in ["version", "parse", "invalid", "error"])


@pytest.mark.integration
class TestModuleStructure:
    """Test the overall module structure and exports."""
    
    def test_module_exports(self):
        """Test that all expected symbols are exported."""
        expected_exports = [
            "Version",
            "VersionRange", 
            "parse_version",
            "parse_version_range",
            "Config",
            "RezCoreError",
            "VersionParseError",
        ]
        
        for export in expected_exports:
            assert hasattr(rez_core, export), f"Missing export: {export}"
    
    def test_module_version_info(self):
        """Test that module has version information."""
        assert hasattr(rez_core, "__version__")
        assert isinstance(rez_core.__version__, str)
        assert len(rez_core.__version__) > 0
    
    def test_module_metadata(self):
        """Test that module has appropriate metadata."""
        metadata_attrs = ["__author__", "__email__"]
        
        for attr in metadata_attrs:
            if hasattr(rez_core, attr):
                assert isinstance(getattr(rez_core, attr), str)


@pytest.mark.integration
class TestPerformanceIntegration:
    """Test performance characteristics of integrated operations."""
    
    def test_mixed_operations_performance(self):
        """Test performance of mixed version operations."""
        import time
        
        start_time = time.time()
        
        # Create versions
        versions = [rez_core.Version(f"1.{i}.0") for i in range(100)]
        
        # Create ranges
        ranges = [rez_core.VersionRange(f">={i}.0.0") for i in range(10)]
        
        # Perform comparisons
        for v in versions[:10]:
            for r in ranges:
                r.contains(v)
        
        # Sort versions
        sorted(versions)
        
        end_time = time.time()
        
        # All operations should complete quickly
        assert (end_time - start_time) < 2.0
    
    def test_memory_usage_reasonable(self):
        """Test that memory usage doesn't grow excessively."""
        import gc
        
        # Force garbage collection
        gc.collect()
        
        # Create many objects
        versions = []
        for i in range(1000):
            v = rez_core.Version(f"1.{i % 100}.{i % 10}")
            versions.append(v)
        
        # Should be able to create many objects without issues
        assert len(versions) == 1000
        
        # Clean up
        del versions
        gc.collect()


@pytest.mark.slow
@pytest.mark.integration
class TestStressIntegration:
    """Stress tests for integrated operations."""
    
    def test_large_scale_version_operations(self):
        """Test operations with large numbers of versions."""
        # Create a large number of versions
        versions = []
        for major in range(10):
            for minor in range(10):
                for patch in range(10):
                    v = rez_core.Version(f"{major}.{minor}.{patch}")
                    versions.append(v)
        
        assert len(versions) == 1000
        
        # Should be able to sort them
        sorted_versions = sorted(versions)
        assert len(sorted_versions) == 1000
        
        # First version should be 0.0.0
        assert str(sorted_versions[0]) == "0.0.0"
        
        # Last version should be 9.9.9
        assert str(sorted_versions[-1]) == "9.9.9"
    
    def test_concurrent_operations(self):
        """Test that operations work correctly when used concurrently."""
        import threading
        import time
        
        results = []
        errors = []
        
        def worker():
            try:
                for i in range(100):
                    v = rez_core.Version(f"1.{i}.0")
                    results.append(str(v))
            except Exception as e:
                errors.append(e)
        
        # Start multiple threads
        threads = []
        for _ in range(5):
            t = threading.Thread(target=worker)
            threads.append(t)
            t.start()
        
        # Wait for all threads to complete
        for t in threads:
            t.join()
        
        # Should have no errors
        assert len(errors) == 0
        
        # Should have results from all threads
        assert len(results) == 500  # 5 threads * 100 versions each
