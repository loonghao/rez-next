#!/usr/bin/env python3
"""
Baseline Update Script

Updates performance baselines with new benchmark results.
"""

import argparse
import json
import shutil
import sys
from datetime import datetime
from pathlib import Path
from typing import Dict, List


class BaselineManager:
    """Manages performance baselines"""

    def __init__(self):
        self.metadata_file = "baseline_metadata.json"

    def load_metadata(self, baseline_dir: Path) -> Dict:
        """Load baseline metadata"""
        metadata_path = baseline_dir / self.metadata_file

        if metadata_path.exists():
            try:
                with open(metadata_path) as f:
                    return json.load(f)
            except Exception as e:
                print(f"Warning: Failed to load metadata: {e}")

        return {
            "version": "1.0.0",
            "created_at": datetime.now().isoformat(),
            "baselines": {},
        }

    def save_metadata(self, baseline_dir: Path, metadata: Dict):
        """Save baseline metadata"""
        metadata_path = baseline_dir / self.metadata_file

        try:
            with open(metadata_path, "w") as f:
                json.dump(metadata, f, indent=2)
        except Exception as e:
            print(f"Error: Failed to save metadata: {e}")
            sys.exit(1)

    def backup_existing_baselines(self, baseline_dir: Path) -> Path:
        """Create backup of existing baselines"""
        if not baseline_dir.exists():
            return None

        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        backup_dir = baseline_dir.parent / f"baseline_backup_{timestamp}"

        try:
            shutil.copytree(baseline_dir, backup_dir)
            print(f"Created backup at {backup_dir}")
            return backup_dir
        except Exception as e:
            print(f"Warning: Failed to create backup: {e}")
            return None

    def validate_benchmark_data(self, file_path: Path) -> bool:
        """Validate benchmark data format"""
        try:
            with open(file_path) as f:
                data = json.load(f)

            # Basic validation
            if not isinstance(data, dict):
                return False

            # Check for expected structure
            if "results" in data:
                # Criterion format
                if not isinstance(data["results"], list):
                    return False

                for result in data["results"]:
                    if not isinstance(result, dict):
                        return False
                    if "mean" not in result or "estimate" not in result["mean"]:
                        return False
            else:
                # Custom format - basic check
                for key, value in data.items():
                    if isinstance(value, dict) and "mean_time_ns" not in value:
                        continue  # Skip non-benchmark entries

            return True

        except Exception as e:
            print(f"Validation failed for {file_path}: {e}")
            return False

    def copy_benchmark_files(self, source_dir: Path, target_dir: Path) -> List[str]:
        """Copy benchmark files from source to target directory"""
        target_dir.mkdir(parents=True, exist_ok=True)

        copied_files = []

        for file_path in source_dir.glob("*.json"):
            if file_path.name == self.metadata_file:
                continue  # Skip metadata file

            if not self.validate_benchmark_data(file_path):
                print(f"Warning: Skipping invalid benchmark file {file_path}")
                continue

            target_path = target_dir / file_path.name

            try:
                shutil.copy2(file_path, target_path)
                copied_files.append(file_path.name)
                print(f"Copied {file_path.name}")
            except Exception as e:
                print(f"Error copying {file_path}: {e}")

        return copied_files

    def update_baselines(
        self,
        benchmark_dir: Path,
        baseline_dir: Path,
        commit_hash: str,
        force: bool = False,
    ) -> bool:
        """Update baselines with new benchmark data"""

        if not benchmark_dir.exists():
            print(f"Error: Benchmark directory {benchmark_dir} does not exist")
            return False

        # Load existing metadata
        metadata = self.load_metadata(baseline_dir)

        # Create backup if baselines exist and not forcing
        if baseline_dir.exists() and not force:
            backup_dir = self.backup_existing_baselines(baseline_dir)
            if backup_dir:
                metadata.setdefault("backups", []).append(
                    {
                        "timestamp": datetime.now().isoformat(),
                        "path": str(backup_dir),
                        "reason": "automatic_backup_before_update",
                    }
                )

        # Copy new benchmark files
        copied_files = self.copy_benchmark_files(benchmark_dir, baseline_dir)

        if not copied_files:
            print("Warning: No valid benchmark files found to copy")
            return False

        # Update metadata
        update_info = {
            "timestamp": datetime.now().isoformat(),
            "commit_hash": commit_hash,
            "files_updated": copied_files,
            "source_directory": str(benchmark_dir),
        }

        metadata["last_updated"] = update_info
        metadata.setdefault("update_history", []).append(update_info)

        # Keep only last 10 updates in history
        if len(metadata["update_history"]) > 10:
            metadata["update_history"] = metadata["update_history"][-10:]

        # Update baseline entries
        for file_name in copied_files:
            module_name = file_name.replace(".json", "")
            metadata.setdefault("baselines", {})[module_name] = {
                "file": file_name,
                "updated_at": update_info["timestamp"],
                "commit_hash": commit_hash,
            }

        # Save updated metadata
        self.save_metadata(baseline_dir, metadata)

        print(f"Successfully updated {len(copied_files)} baseline files")
        return True

    def list_baselines(self, baseline_dir: Path):
        """List current baselines"""
        if not baseline_dir.exists():
            print("No baselines directory found")
            return

        metadata = self.load_metadata(baseline_dir)

        print("Current Baselines:")
        print("=" * 50)

        if "baselines" in metadata:
            for module_name, info in metadata["baselines"].items():
                print(f"Module: {module_name}")
                print(f"  File: {info['file']}")
                print(f"  Updated: {info['updated_at']}")
                print(f"  Commit: {info['commit_hash']}")
                print()
        else:
            print("No baselines found")

        if "last_updated" in metadata:
            print(f"Last update: {metadata['last_updated']['timestamp']}")
            print(f"From commit: {metadata['last_updated']['commit_hash']}")

    def clean_old_backups(self, baseline_dir: Path, keep_count: int = 5):
        """Clean old backup directories"""
        if not baseline_dir.exists():
            return

        metadata = self.load_metadata(baseline_dir)
        backups = metadata.get("backups", [])

        if len(backups) <= keep_count:
            return

        # Sort by timestamp and keep only the most recent
        backups.sort(key=lambda x: x["timestamp"], reverse=True)
        backups_to_remove = backups[keep_count:]

        for backup in backups_to_remove:
            backup_path = Path(backup["path"])
            if backup_path.exists():
                try:
                    shutil.rmtree(backup_path)
                    print(f"Removed old backup: {backup_path}")
                except Exception as e:
                    print(f"Warning: Failed to remove backup {backup_path}: {e}")

        # Update metadata
        metadata["backups"] = backups[:keep_count]
        self.save_metadata(baseline_dir, metadata)


def main():
    parser = argparse.ArgumentParser(description="Update performance baselines")
    parser.add_argument(
        "--benchmark-dir",
        type=Path,
        required=True,
        help="Directory containing new benchmark results",
    )
    parser.add_argument(
        "--baseline-dir",
        type=Path,
        required=True,
        help="Directory to store baseline results",
    )
    parser.add_argument(
        "--commit-hash", required=True, help="Git commit hash for this update"
    )
    parser.add_argument(
        "--force", action="store_true", help="Force update without creating backup"
    )
    parser.add_argument(
        "--list", action="store_true", help="List current baselines and exit"
    )
    parser.add_argument(
        "--clean-backups",
        type=int,
        metavar="N",
        help="Clean old backups, keeping only N most recent",
    )

    args = parser.parse_args()

    manager = BaselineManager()

    if args.list:
        manager.list_baselines(args.baseline_dir)
        return

    if args.clean_backups is not None:
        manager.clean_old_backups(args.baseline_dir, args.clean_backups)
        return

    # Update baselines
    success = manager.update_baselines(
        args.benchmark_dir, args.baseline_dir, args.commit_hash, args.force
    )

    if success:
        print("Baseline update completed successfully")

        # Clean old backups automatically
        manager.clean_old_backups(args.baseline_dir)
    else:
        print("Baseline update failed")
        sys.exit(1)


if __name__ == "__main__":
    main()
