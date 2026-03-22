# [1.3.0](https://github.com/treble-app/cli/compare/v1.2.1...v1.3.0) (2026-03-22)


### Bug Fixes

* correct plugin install instructions in README ([fa2c06a](https://github.com/treble-app/cli/commit/fa2c06aa0ef268f80120c7786a2f7ebd31acc533))


### Features

* detect Claude Code plugin in treble init ([d43a277](https://github.com/treble-app/cli/commit/d43a277b90348ada350feee4b97e1e7dc0b2af0a))

## [1.2.1](https://github.com/treble-app/cli/compare/v1.2.0...v1.2.1) (2026-03-22)


### Bug Fixes

* add visual pipeline preflight and stronger mission to dev prompt ([f93b30e](https://github.com/treble-app/cli/commit/f93b30e7bc69f260860f0282bb8a204934b79a77))

# [1.2.0](https://github.com/treble-app/cli/compare/v1.1.11...v1.2.0) (2026-03-21)


### Features

* multi-account Figma auth with login --as, logout, and project bindings ([51476b9](https://github.com/treble-app/cli/commit/51476b9c47439625322fa2d7822c2556a349498e))

## [1.1.11](https://github.com/treble-app/cli/compare/v1.1.10...v1.1.11) (2026-03-21)


### Bug Fixes

* PAT login fails when stale OAuth fields exist ([a9852eb](https://github.com/treble-app/cli/commit/a9852eb88680b8a8a9d2c0df7b3a3216f9776cc2))

## [1.1.10](https://github.com/treble-app/cli/compare/v1.1.9...v1.1.10) (2026-03-12)


### Bug Fixes

* add discovery guard rail to sync prompt ([25e6f57](https://github.com/treble-app/cli/commit/25e6f573af174cf42092d82586c6afd7400bdd91))

## [1.1.9](https://github.com/treble-app/cli/compare/v1.1.8...v1.1.9) (2026-03-12)


### Bug Fixes

* add guard rails to dev prompt for CMS scope, Docker, and page selection ([456b8fb](https://github.com/treble-app/cli/commit/456b8fb2b29f8f875f6f3d8cd8ef15dec552a842))

## [1.1.8](https://github.com/treble-app/cli/compare/v1.1.7...v1.1.8) (2026-03-10)


### Bug Fixes

* correct skill frontmatter and hand-off invocation ([e167239](https://github.com/treble-app/cli/commit/e16723975091b1409a0b4906d1d7facca1e475cb))
* increase parallel plan subagents from 3 to 5 ([cd7f2a9](https://github.com/treble-app/cli/commit/cd7f2a91d84567999a711aa004bfcba2c951f054))

## [1.1.7](https://github.com/treble-app/cli/compare/v1.1.6...v1.1.7) (2026-03-10)


### Bug Fixes

* restructure skills to {name}/SKILL.md for plugin discovery ([08ce96d](https://github.com/treble-app/cli/commit/08ce96da030ec56c9c6134766167eb7d4f904fde))

## [1.1.6](https://github.com/treble-app/cli/compare/v1.1.5...v1.1.6) (2026-03-10)


### Bug Fixes

* add layer composition analysis to plan→build pipeline ([9620f50](https://github.com/treble-app/cli/commit/9620f50faa4548e09228222f1ad2e4ff1e535d47))

## [1.1.5](https://github.com/treble-app/cli/compare/v1.1.4...v1.1.5) (2026-03-10)


### Bug Fixes

* enforce parallel subagent spawning in plan command ([3830c39](https://github.com/treble-app/cli/commit/3830c39e70294a08d131cbacf085c05f5beffba1))

## [1.1.4](https://github.com/treble-app/cli/compare/v1.1.3...v1.1.4) (2026-03-08)


### Bug Fixes

* cargo fmt formatting in init.rs ([1061114](https://github.com/treble-app/cli/commit/1061114aad4ba4b547e7d88e0d0c414d87292bb0))

## [1.1.3](https://github.com/treble-app/cli/compare/v1.1.2...v1.1.3) (2026-03-08)


### Bug Fixes

* Use Bearer auth for OAuth tokens, X-Figma-Token for PATs ([d81e438](https://github.com/treble-app/cli/commit/d81e4380f4ab6977316b27ad1ce639d1d4100fd8))

## [1.1.2](https://github.com/treble-app/cli/compare/v1.1.1...v1.1.2) (2026-03-08)


### Bug Fixes

* set correct version in binary before build ([7755485](https://github.com/treble-app/cli/commit/7755485e5621e7e171fcc3f85546f1a41e02b43d))

## [1.1.1](https://github.com/treble-app/cli/compare/v1.1.0...v1.1.1) (2026-03-08)


### Bug Fixes

* npm ci --omit=optional to skip missing platform packages ([564e077](https://github.com/treble-app/cli/commit/564e0770b117c4c41dbaae5f9ce26e4dfb5c8cb6))

# [1.1.0](https://github.com/treble-app/cli/compare/v1.0.0...v1.1.0) (2026-03-08)


### Bug Fixes

* include scripts/ in npm package files ([53b67e9](https://github.com/treble-app/cli/commit/53b67e9f81f7c14df6d3834655b929f6abde0a1d))
* remove postinstall hook, restore npm ci in release ([d0807e3](https://github.com/treble-app/cli/commit/d0807e3b5f64e5f91008111add52e45d85a03973))
* rename marketplace from treble-build to treble-app ([9854d48](https://github.com/treble-app/cli/commit/9854d487598514ff2e875d3f8cc0197044ff3b14))
* use npm install instead of npm ci in release workflow ([417e942](https://github.com/treble-app/cli/commit/417e94207c275e18b99584300d0090b1a6c53225))


### Features

* add /treble:sync command and treble status CLI ([be71004](https://github.com/treble-app/cli/commit/be71004068cdca263be978625842f37c6b9a967e))

# 1.0.0 (2026-03-08)


### Bug Fixes

* add package-lock.json for CI (npm ci requires it) ([68edaf7](https://github.com/treble-app/treble-cli/commit/68edaf78dbdd92aca9d9b4d321c3a8cf57d9625e))
* align cms-wp visual verification with dev-shadcn format ([b4639ed](https://github.com/treble-app/treble-cli/commit/b4639ede854300fe5ac75c8b1a62a23908303240))
* show Figma settings URL for PAT generation in login flow ([afd5ed7](https://github.com/treble-app/treble-cli/commit/afd5ed7573daf1ba7e5ab7932207e913eb62a63c))


### Features

* add /treble:cms router — routes to platform-specific CMS skill ([ec0c5dd](https://github.com/treble-app/treble-cli/commit/ec0c5dde28f803caec683fc9c9fbb66b338b7db2))
* add cms-wp prompt + CLAUDE.md updates ([1714ec7](https://github.com/treble-app/treble-cli/commit/1714ec78f39d112cc42770210f20943f6826a2d9))
* add Prismic and Sanity CMS skills, update router ([5c30fb2](https://github.com/treble-app/treble-cli/commit/5c30fb2cc7a6fb88923431202b7f0fa01ea20295))
* add Step 0 project setup to /treble:dev build loop ([a17bbca](https://github.com/treble-app/treble-cli/commit/a17bbca9da7d3d213e62d02e6c75e7f1f07b3110))
* feature-based architecture + Next.js/Astro choice in build loop ([ee21a89](https://github.com/treble-app/treble-cli/commit/ee21a89f779950d284ada192cce4a3fe49a45fa6))
* npm distribution scaffolding for @treble-app/cli ([0393062](https://github.com/treble-app/treble-cli/commit/03930622d49522d43b0f9a0b8d8efbd8ec1e64c3))
* plan.md slicing strategy, tool guardrails, messy file handling ([bc66dda](https://github.com/treble-app/treble-cli/commit/bc66dda4fa15c0ba34c67471e2a5f464e58286f7))
* responsive strategy, commercial font flags, SVG extraction prompts ([fec8578](https://github.com/treble-app/treble-cli/commit/fec8578c8b64b06c645a0346a77ac555ec3618c9))
* rewrite cms-wp as multi-approach prompt with Gutenberg blocks ([2632f4d](https://github.com/treble-app/treble-cli/commit/2632f4dd662ac0dc1767c5cdb16ccf04bec9c3f6))
* robust build pipeline — projectSetup, tailwindClasses, error recovery ([9ec2938](https://github.com/treble-app/treble-cli/commit/9ec29381d7be728ebb38650cb5062376ef212736))
* smart triage in /treble:dev, compatibility gating in /treble:cms ([5090858](https://github.com/treble-app/treble-cli/commit/5090858282cfd5a791e0926e28c43a597ba83830))
* split build loop into flavor-specific commands ([d48b0a7](https://github.com/treble-app/treble-cli/commit/d48b0a740415c2cdfc1c0e1be06b39d625ee4d85))
* treble Claude Code plugin — Figma-to-code build assistant ([d62fbb7](https://github.com/treble-app/treble-cli/commit/d62fbb7086cd40966cdffcfdd63140ce69318626))
* treble CLI — sync, tree, show with batching and slicing ([5a8637f](https://github.com/treble-app/treble-cli/commit/5a8637fe88e5f70bc5af242a64c7fe2f80ec3818))
* treble extract — source image extraction from Figma IMAGE fills ([73117aa](https://github.com/treble-app/treble-cli/commit/73117aa6202471695c4b1bd796c7a174cd3588c6))
* two-tier visual verification in cms-wp prompt ([f5c7b85](https://github.com/treble-app/treble-cli/commit/f5c7b85bdb709077a7b551b21de5f4b95142d036))
* zoom-in workflow, referenceImages, treble show --json ([dad819b](https://github.com/treble-app/treble-cli/commit/dad819b4a252efd9a2482d2f1bb81d9802078b72))
