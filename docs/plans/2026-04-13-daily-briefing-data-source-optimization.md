# Daily Briefing Data-Source Optimization Plan

> For Hermes: optimize the cron workflow in phases; review the source architecture with the user before updating the cron job.

Goal: Upgrade the `daily-ai-crypto-briefing` cron job so it captures newer, more valuable inputs: open-source AI/agent projects, crypto protocol and product updates, emerging "dark horse" crypto/web3 projects, and produces a more analytical Chinese briefing instead of a mostly headline-style digest.

Architecture: Keep the current `daily-ai-crypto-briefing` skill as the execution backbone, but narrow and prioritize the data sources around four explicit research tracks: AI/company news, open-source AI & agent projects, crypto market/news, and crypto/web3 emerging projects. The cron prompt should become shorter, while the skill or attached workflow carries the richer source hierarchy and analysis rules.

Tech Stack: Hermes cronjob, `daily-ai-crypto-briefing` skill, browser/web/search tools, optional social sources, and source-tiered fallbacks.

---

## What the user wants optimized

The user explicitly wants the briefing to emphasize:
1. 更“新”的 update（近 24h 优先，必要时近 48h）
2. 开源项目更新，尤其是 AI / Agent / infra
3. crypto 项目更新
4. 新的黑马 crypto / web3 项目
5. 好的 AI 开源项目，尤其 Agent 方向
6. 生成文章时做更细致的分析，而不是只堆新闻点

This means the cron job should stop behaving like a generic market/news scraper and become a curated analyst-style pipeline.

---

## Target output structure (new)

The upgraded briefing should shift from broad equal-weight sections to weighted sections:

1. *Big Picture / Big News*
   - Cross-market items that matter for both AI and crypto
   - 只保留最重要的 3-5 条

2. *Open-Source AI & Agent Watch*
   - New repo launches
   - Major version releases
   - Material GitHub velocity changes (stars / forks / issue activity / trend signal)
   - Why it matters: infra, workflow, model serving, agent orchestration, evals, tool use, browser agents

3. *Crypto / Web3 Project Watch*
   - Official protocol updates
   - Funding / token / ecosystem / product releases
   - Infrastructure and app-layer releases
   - "黑马项目观察池": new, fast-rising, but clearly separated into high-quality vs high-risk

4. *Market + Narrative Analysis*
   - US AI supply-chain equities
   - BTC / ETH / SOL baseline
   - Then narrative-level interpretation: where attention is concentrating and why

5. *Analyst Notes*
   - 不是简单罗列，而是总结：
     - 哪些更新是短期噪音
     - 哪些更新可能有中期影响
     - 哪些开源项目值得持续跟踪
     - 哪些 crypto/web3 项目属于高风险观察，不宜过度放大

---

## New source architecture

### Tier A: Highest priority, must prefer first

These are the best sources for freshness + signal quality.

#### A1. Official AI / OSS project sources
- GitHub Releases pages
- GitHub repository activity pages / trending signals
- Official project blogs / changelogs / docs release notes
- Official X/Twitter accounts of major OSS AI projects
- Official Discord/Docs changelog pages if public

Primary use cases:
- New OSS agent framework release
- New model-serving infra release
- New eval / browser / tool-use framework launch
- Important open-source model or agent capability updates

#### A2. Official crypto / web3 project sources
- Official blogs
- Official docs / changelog / governance forum
- Official X/Twitter announcements
- Foundation or protocol updates
- Product launch pages / ecosystem dashboards

Primary use cases:
- Protocol upgrades
- Mainnet / testnet launches
- Wallet / infra / DeFi / AI+crypto product releases
- Incentive / ecosystem / partner announcements

#### A3. High-credibility media for context
- Reuters
- CoinDesk
- The Block
- Bloomberg / WSJ / FT where accessible
- Major mainstream business coverage for AI infra / policy / enterprise moves

Primary use cases:
- Cross-checking significance
- Policy / regulation / funding / macro interpretation

---

### Tier B: Discovery sources for emerging winners

These are useful for finding new projects, but they should not be treated as enough by themselves.

#### B1. GitHub discovery
- GitHub Trending
- Search by topics: `agent`, `browser-agent`, `ai-agent`, `tool-use`, `multi-agent`, `evals`, `inference`, `rag`, `oss-llm`
- Recent repo creation + fast star growth

What to look for:
- unusually fast traction
- strong commit cadence
- credible maintainers / orgs
- actual README/demo/docs quality

#### B2. Crypto discovery
- CoinGecko trending / categories / recently hot pages
- Official ecosystem grant announcements
- VC / incubator / accelerator announcements
- High-signal launchpads only if there is official corroboration

What to look for:
- real product, not just token narrative
- credible team / funding / ecosystem support
- onchain or usage indicators if accessible

#### B3. Social signal layers
- Twitter/X themes
- Reddit themes
- Hacker News / Product Hunt if relevant for OSS AI

Use only as:
- attention map
- sentiment signal
- idea discovery

Not sufficient alone for inclusion.

---

### Tier C: Fallback-only sources

Use these only when Tier A/B fail or to supplement context.
- Google News indexed pages
- general web search summaries
- mirror blogs / reposts
- social repost screenshots

Rule:
- if an item only exists in Tier C, mark it `待验证` or exclude it.

---

## New coverage buckets

### Bucket 1: Open-source AI / Agent projects

This should become a first-class section, not a side note.

Track these subtypes:
- Agent frameworks
- Browser agents
- Tool-use / MCP / workflow orchestration projects
- Eval / benchmark / observability tools for agents
- Open-source model-serving / inference / finetuning infra
- New open-source models with practical agent relevance

Selection rule:
Include only projects that meet at least 2 of these:
- notable release or launch in last 24-48h
- meaningful GitHub traction or update velocity
- practical relevance to agent workflows
- credible maintainer/team
- evidence of adoption or community attention

For each included project, the writeup should answer:
- What changed?
- Why does it matter?
- Is it real infrastructure value or just hype?
- Who should care?

### Bucket 2: Crypto / web3 project updates

Track:
- protocol upgrades
- infrastructure changes
- major ecosystem integrations
- launches in wallets / exchanges / DeFi / infra / data / AI+crypto
- governance decisions if material

For each included update, answer:
- Is it product progress, token narrative, or policy-driven move?
- Does it change usage/adoption potential?
- Is it likely short-term attention or medium-term value creation?

### Bucket 3: Dark-horse crypto / web3 projects

This section should be explicitly split into two lists:
1. `高质量观察池`
2. `高风险高弹性池`

A project can enter `高质量观察池` if it has:
- official launch/update evidence
- a real product or infra angle
- credible team/ecosystem support
- some measurable adoption / distribution / traction signal

A project goes to `高风险高弹性池` if it has:
- fast attention spike
- weak verification
- token-driven narrative dominating product signal
- meme / microcap / launch-stage volatility

This separation is important so the report stays useful and doesn’t amplify junk equally.

---

## New writing rules for the generated briefing

The article should become more analytical.

### Rule 1: Every major item needs a mini-analysis
Instead of:
- Project X launched Y

Use:
- Project X launched Y. The key point is Z. This matters because A. Near-term impact is likely B, but the risk is C.

### Rule 2: Add comparative judgment
For OSS AI/agents:
- compare against existing tools / category norms
- say whether it looks incremental or category-shifting

For crypto:
- say whether it is infra progress, liquidity theater, or genuine adoption progress

### Rule 3: Separate signal from noise
Each section should implicitly classify:
- high conviction signal
- medium conviction but worth watching
- speculative / high risk

### Rule 4: Reduce filler headlines
If an item has no analytical value, omit it even if it is recent.

### Rule 5: Prefer fewer but richer items
Better:
- 5 strong items with reasoning
Than:
- 15 shallow items with no insight

---

## Step-by-step execution plan

### Step 1: Lock the new source priorities with the user
Review and confirm these priorities:
- Open-source AI / Agent updates become first-class
- Crypto project updates become first-class
- Dark-horse crypto/web3 gets split into quality vs high-risk
- Article style shifts to analyst notes, not just headlines

Deliverable:
- approved source architecture and section priorities

### Step 2: Update the skill / workflow rules
After approval, patch the `daily-ai-crypto-briefing` skill so it explicitly tells the cron run to:
- search official OSS project sources and GitHub first for open-source AI/Agent updates
- search official crypto protocol/project sources first for project updates
- use social only as discovery, not primary evidence
- generate richer analysis per item

Deliverable:
- updated skill instructions

### Step 3: Shorten and refocus the cron prompt
The cron prompt should stop repeating generic rules already in the skill.
It should instead emphasize the custom weighting:
- prioritize OSS AI/Agent
- prioritize crypto/web3 project updates
- track dark horses carefully
- prefer fewer but more analytical items

Deliverable:
- shorter, sharper cron prompt

### Step 4: Test one manual run
Manually run the cron job or a one-shot clone and inspect:
- did it pick up fresher OSS/crypto project updates?
- did it overuse market filler?
- did dark-horse picks stay disciplined?
- did the prose become more analytical?

Deliverable:
- one reviewed sample briefing

### Step 5: Iterate source lists
Based on the test output:
- add missing high-signal sources
- demote noisy sources
- refine project inclusion thresholds

Deliverable:
- stabilized source pipeline

---

## Immediate recommendation

For the next step, do not change the cron job yet.
First lock this source architecture with the user.

Specifically, ask the user to confirm these four priorities:
1. 是否把 `Open-source AI & Agent Watch` 提到核心位置
2. 是否把 `Crypto / Web3 Project Watch` 提到核心位置
3. 黑马项目是否固定拆成 `高质量观察池` 和 `高风险高弹性池`
4. 简报是否接受“条目更少，但每条分析更深”的风格

Once those are confirmed, update the skill first, then the cron prompt.
