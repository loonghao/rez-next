# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-01-22

### Added
- Initial release of rez-core-repository
- High-performance repository management and scanning
- Parallel package discovery with intelligent caching
- Repository manager for multi-repository coordination
- Advanced caching system with TTL and LRU eviction
- Comprehensive repository operations and management
- Full compatibility with original Rez repository semantics

### Features
- **Repository Scanning**: High-performance parallel scanning of package repositories
- **Package Discovery**: Intelligent package discovery with caching and indexing
- **Repository Management**: Multi-repository management and coordination
- **Caching**: Advanced caching with TTL, LRU eviction, and performance optimization
- **Performance**: Optimized for high-throughput scanning operations
- **Compatibility**: 100% compatible with Rez repository structure and semantics

### Components
- **Repository**: Core repository type with scanning and management capabilities
- **RepositoryManager**: Multi-repository management and coordination
- **RepositoryScanner**: High-performance parallel scanning engine
- **RepositoryCache**: Intelligent caching with configurable policies
- **ScanOptions**: Flexible scanning configuration and optimization
- **CacheOptions**: Comprehensive caching configuration

### Performance Features
- Parallel repository scanning for maximum throughput
- Intelligent caching and indexing for fast repeated operations
- Memory-efficient data structures and minimal allocations
- Async I/O for non-blocking repository operations
- Configurable scanning options for different use cases

[Unreleased]: https://github.com/loonghao/rez-core/compare/rez-core-repository-v0.1.0...HEAD
[0.1.0]: https://github.com/loonghao/rez-core/releases/tag/rez-core-repository-v0.1.0
