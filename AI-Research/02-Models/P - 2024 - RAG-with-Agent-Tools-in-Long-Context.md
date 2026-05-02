type: paper
status: draft
date: 2024-01-15
tags: [LLM, Agent]
ai_generated: true
------------------

# RAG with Agent Tools in Long Context

**Source:** ARXIV: 2601.00155
**Authors:** Alice Smith
**Published:** 2024-01-15 | **Updated:** 2024-01-15
**Landing:** https://arxiv.org/abs/2601.00155
**PDF:** https://arxiv.org/pdf/2601.00155.pdf
**Primary Category:** N/A

---

## Research Question Card

* 我想解决什么问题？
* 为什么重要？
* 我的先验判断是什么？
* 什么证据会推翻我？

---

## 1. 背景

> **Abstract（原文）**
> Retrieval augmented generation with agent tools for long context.

> **Abstract（原文）**
> Retrieval augmented generation with agent tools for long context.

_（关键词匹配摘要，建议 AI 深入分析）_

---

## 2. 核心问题

_基于摘要推断：_

> _（需要 AI 分析）_

---

## 3. 方法结构
### 3.1 架构拆解

_检测到的方法/架构关键词：attention, normalization_



> _（需要 AI 分析）_

### 3.2 算法逻辑



> _（需要 AI 分析）_

### 3.3 关键组件



> _（需要 AI 分析）_

---

## 4. 关键创新



> _（需要 AI 分析）_

---

## 5. 实验分析
### 5.1 数据集



> _（需要 AI 分析）_

### 5.2 基线对比

_检测到的评估指标关键词：recall, map, auc_



> _（需要 AI 分析）_

### 5.3 消融实验



> _（需要 AI 分析）_

### 5.4 成本分析



> _（需要 AI 分析）_

---

## 6. 对抗式审稿
* 逻辑漏洞：
* 偏置风险：
* 复现难度：
* 失败模式推测：



> _（需要 AI 分析）_

---

## 7. 优势



> _（需要 AI 分析）_

---

## 8. 局限



> _（需要 AI 分析）_

---

## 9. 本质抽象



> _（需要 AI 分析）_

---

## 10. 与其他方法对比
* vs A：
* vs B：
* vs C：



> _（需要 AI 分析）_

---

## 11. Decision（决策）
* 是否使用？
* 使用场景？
* 不适用边界？
* 接下来关注信号？



> _（需要 AI 分析）_

---

## 知识蒸馏
### Facts
1.
2.

### Principles
1.
2.

### Insights
1.
2.



> _（需要 AI 分析）_

---

## 认知升级
* 长期价值：
* 规模效应：
* 技术护城河：
* 是否范式转移：
* 商业潜力：



> _（需要 AI 分析）_

---

## 评分量表



* Novelty (1-5):
* Leverage (1-5):
* Evidence (1-5):
* Cost (1-5):
* Moat (1-5):
* Adoption Signal (1-5):

### Overall Judgment



---

## 附：PDF 章节粗拆（自动抽取 · 供快速定位）

Dynamic Market Design1
Yeon-Koo Che2
January 5, 2026
Abstract
Classic market design theory is rooted in static models where all participants
trade simultaneously. In contrast, modern platform-mediated digital markets are
fundamentally dynamic, defined by the asynchronous and stochastic arrival of sup-
ply and demand. This chapter surveys recent work that brings market design to
this dynamic setting. We focus on a methodological framework that transforms
complex dynamic problems into tractable static programs by analyzing the long-
run stationary distribution of the system. The survey explores how priority rules
and information policy can be designed to clear markets and screen agents when
monetary transfers are unavailable, and, when they are available, how queues of
participants and goods can be managed to balance intertemporal mismatches of
demand and supply and to spread competitive pressures across time.
JEL Classification Numbers: C78, C61, D47, D83, D61
Keywords: Asynchronous and stochastic arrival of participants, steady state mar-
ket design, non-transferable utility models, transferable utility models.
1This is a survey prepared for an invited lecture at the 2025 World Congress of the Econometric
Society. I am grateful to Dirk Bergemann, Andy Choi, and Sangjun Park, for their comments.
2Department of Economics, Columbia University, USA. Email: yeonkooche@gmail.com.
1
arXiv:2601.00155v1  [econ.TH]  1 Jan 2026

1
Introduction
The basic workhorse models of market design are rooted in two great traditions. For
markets mediated by monetary prices, or Transferable Utility (TU) markets, the foun-
dational theories of auctions by Myerson (1981) and Milgrom and Weber (1982) and
matching theories of Shapley and Shubik (1971) and Becker (1973) provide our core
understanding. For markets coordinated without direct transfers, or Non-Transferable
Utility (NTU) markets, the canonical matching model of Gale and Shapley (1962) is
the foundational paradigm. These seminal models, despite their power, reflect a classic
market setting where all buyers and sellers must be present simultaneously at the same
location to execute trades.
The advances in information technology and the internet have fundamentally broken
down these physical and temporal barriers. The defining feature of the resulting modern
marketplaces is that participants—buyers and sellers, or agents and goods—arrive over
time asynchronously and stochastically. A typical transaction today is mediated by
a digital platform that must manage unpredictable, often mismatched flows of supply and
demand. These platforms do not merely serve as passive meeting grounds; they actively
structure the market by matching or recommending participants to one another, in many
cases deciding who must wait for a match and for how long.
Examples of such dynamic markets are ubiquitous. In the NTU setting, asynchronous
and stochastic arrivals figure prominently in kidney exchange, public housing, ride-
hailing,1 and professional job matching. In the TU setting, examples include the sale
of cloud computing capacity, where demand for server time arrives and supply is freed up
as old jobs complete; gig platforms like TaskRabbit, which face unpredictable availability
of both service providers and customer requests; and blockchains, which must allocate
randomly arriving transactions to blocks that are themselves generated at random inter-
1While ride-hailing platforms often employ dynamic (surge) pricing, the actual allocation or dispatch
of a specific rider to a driver is typically mediated by proximity and arrival order rather than a price-
based auction for each match. Rare exceptions exist, such as the Czech platform Liftago, which uses
explicit auctions to mediate dispatch; however, for the vast majority of platforms, the allocation decision
remains better classified as an NTU case.
2

vals.
The goal of this paper is to survey recent works that bring the classical agenda of
market and mechanism design up to date by placing it in this more realistic dynamic
setting. The relevant literature is already voluminous and growing rapidly, and we do not
aim to be exhaustive. Instead, we focus on a strand of recent literature, developed largely
around the works of Baccara, Lee, and Yariv (2020), Che and Tercieux (forth.), Madsen
and Shmaya (2025), and Che and Choi (2025), that highlights a particular methodological
framework capable of collapsing the complex dynamics of these markets into a tractable,
static linear programming problem.
The workhorse framework we develop borrows heavily from queueing theory.
For
illustrative clarity, we will ground the survey in a simple M/M/1 environment, where
buyers arrive at a Poisson rate λ and items (or service completions) arrive at a Poisson
rate µ.2 While much of the analysis can be generalized, this setting is ideal for conveying
the core ideas. We consider a general class of mechanisms, called Positive Recurrent
Regenerative Mechanisms (PRRMs), which induce a process for the system’s state that
is positive recurrent and regenerative. This class is general enough to be without loss
for most problems of interest and nests, as a special case, the simpler class of Markovian
mechanisms that condition policies on payoff-relevant states such as the size and types of
agents in the queue. This generality is warranted, as the optimal mechanisms we identify
are often non-Markovian.
Despite their generality, PRRMs retain crucial tractability
because they admit a unique stationary distribution, allowing the designer’s objective
and the agents’ incentives to be evaluated in the long-run steady state.
The central methodology, therefore, is to transform the dynamic design problem into
a static optimization problem: choosing a stationary distribution over the space of queue
states that maximizes the designer’s objective, subject to the constraint that this dis-
tribution must be implementable by some feasible PRRM. This presents two primary
challenges. First, it is not immediately obvious how to characterize the set of feasible
2In Kendall’s notation, M/M/1 denotes a system with Markovian (Poisson) arrivals, Markovian
(exponential) service times, and a single server.
3

stationary distributions. We show how this can be done using a Border-inspired charac-
terization of reduced-form allocations, adapted to a dynamic setup. Second, the space
of queue states can be forbiddingly complex—for instance, including the reported types
of all agents in the queue—making the dimensionality of the stationary distribution in-
tractable. We demonstrate how this dimensionality can be reduced to make the analysis
tractable.
This survey is organized into two parts: NTU and TU markets.
First, we analyze the NTU setting, where goods and services are allocated without
monetary transfers. While other non-monetary mechanisms exist, we focus on the most
common: waiting in line, or “queueing.”3 Though queueing can play a role similar to
pricing in clearing markets and screening types, there are noteworthy distinctions. In
a competitive market, a decision to pay a monetary price has no direct externality on
others. In contrast, a decision to join a queue can impose a significant externality, and
the nature of this externality depends on three key factors: the admissions control policy
(how entry and exit are regulated), the queueing rule (how priority is assigned), and the
information environment (what agents know about the state of the queue). Moreover, the
decision is fundamentally dynamic, as agents are typically free to abandon the queue at
any time. This suggests that special attention must be paid to how queueing incentives
are managed to serve the designer’s objective.
We will survey several key papers that study this problem under complete informa-
tion, summarizing the findings of Naor (1969), Hassin (1985), and Leshno (2022) on the
relative merits of First-Come, First-Served (FCFS), Last-Come, First-Served (LCFS),
and Service-In-Random-Order (SIRO). We then discuss how Che and Tercieux (forth.)
applies the steady-state framework to analyze optimal queue design, showing that FCFS
re-emerges as optimal when the designer has a complete toolkit. Finally, we discuss the
implications of using queueing to screen agents with heterogeneous types. While waiting
can function like a price, it entails wasteful social costs, a crucial distinction from mone-
3Here, we use the term queueing generically to mean any methods that involve agents waiting in
line, and not necessarily to mean a particular queueing rule such as first-come-first-served.
4

tary transfers. This trade-off implies that a welfare-motivated designer may be willing to
sacrifice allocative efficiency to avoid these costs, sometimes resulting in complete pooling
and random allocation as the optimal policy.
Second, we turn to the TU model, where monetary transfers can be used without
restriction. Given the availability of transfers, the wasteful queueing of the NTU world
need not be relied upon for screening or market clearing. Instead, the central question
is how to balance competitive forces across time optimally. This adds an important new
dimension to the static design problem of Myerson (1981): the designer must not only
allocate goods/services optimally among currently present agents but also store buyers
or goods optimally in a queue for potential future allocation. This involves dynamically
managing the entry and exit of buyers into and out of queues based on their types, as
well as managing an inventory of goods. We will then summarize the main results of Che
and Choi (2025), which characterizes the optimal dynamic auction in this setting.
A final section concludes by discussing some topics and literature not covered in the
survey and suggesting directions for future research.
2
Non-Transferable Utility Model
Primitives.
We consider a continuous-time model in which a platform/designer allo-
cates goods or services arriving at the platform to buyers who also arrive at the platform
over time. At each instant t ∈[0, ∞), buyers arrive at a Poisson rate of λ > 0 and
homogeneous goods—or a firm offering the goods— arrive at a Poisson rate of µ > 0. In
the context of the service center, the arrival of goods can be interpreted as the arrival
of “service completions,” which would be enjoyed by a buyer if they have been receiving
service, or would be wasted if there is no buyer receiving service.
Initially, we assume that buyers are also homogeneous; they value a fixed surplus
v > 0 from receiving the good/service, and they incur c ∈(0, v) per unit of time they
5

wait in the queue. There is no discounting.4 This means that, if a buyer waits t ≥0 and
receives the good/service, he enjoys the payoff of v −ct; his payoff is −ct in case he never
receives the good. His outside option is zero.5
The designer’s job is to allocate goods to buyers. In case no item is available when a
buyer arrives, the designer may hold the buyer in a (metaphorical) waiting room, called
a queue, until an item arrives.6 Likewise, if no buyer is available, when an item arrives,
the designer may hold it in inventory until a buyer arrives; however, doing so incurs cost
d > 0 to the designer per unit of time. There is also a firm that provides service or good;
it earns a fixed profit π > 0 per each item or service rendered to a buyer.
One can see that this model encompasses two modal scenarios of modern marketplaces.
• Service scenario: Service centers provide perishable goods or services. Cloud
computing, repair and maintenance services, and customer services all fall into this
category. In the context of the service center, the arrival of goods is interpreted as
the completion of service for buyers who are already receiving service. Specifically,
as long as there is at least one buyer in the system, service is being provided; at the
Poisson rate µ, a service completion occurs. If a service completion ’arrives’ when
there are no buyers (i.e., the system is empty), the service opportunity is wasted.
This is a special case, where d = ∞so that only buyers can wait in a queue, and
items cannot be stored.
• Goods and dynamic matching scenario: Retail platforms, dating apps, public
4It is customary to assume in the queueing literature that the only discounting involves linear waiting
costs, as we assume. There are two reasons. First, exponential discounting introduces risk-loving time
preferences, which are often contrary to what authors believe customers exhibit (e.g., risk-averse time
preferences). For instance, customers are known to exhibit a strong preference for the first-come, first-
served system (FCFS), which has the least dispersed waiting time distribution among all queueing
disciplines. Second, nontrivial time preferences interact with the effects of other policy variables in a
manner that makes it difficult to isolate their effect. Linear waiting times, which imply risk-neutral time
preferences, help isolate the channel of effects orthogonal to those caused by nonlinear time preferences.
5Note that a buyer stops incurring the waiting cost once he leaves the queue. One can think of the
cost as the opportunity cost of not exercising the outside option, which would yield the flow value of c.
An outside option could be a leisure activity that the buyer is forgoing or the next best good or service
he has immediate access to.
6We use the term “queue” in a broad sense without any connotation about the service priority rule,
such as first-come, first-served.
6

housing authority, human organs transplant organizations, child adoptions and fos-
ter care agencies mediate matches between agents/resources on two-sided markets.7
In these environments, entities on both sides can be held in queues if immediate
matches are impossible or undesirable.
Mechanisms.
A canonical probability space (Ω, F, P) captures the primitive arrival
processes. A history ω ∈Ωis a realization of two independent Poisson processes tracking
the arrival times of buyers, {ai(ω)}i∈N, and items, {gj(ω)}j∈N, indexed by their arrival
order. Let {Ft}t≥0 be the natural filtration generated by these processes. A mechanism,
ϕ, is a non-anticipatory,8 measurable mapping from histories to outcomes. It specifies
which buyers and items are queued, when they are removed, and how they are matched.9
We also impose an efficiency condition, No Allocation Delay (NAD), meaning the
mechanism never holds both buyers and items simultaneously—an assumption that is
without loss for the objectives we consider.
A mechanism ϕ induces an outcome y ∈Y at each time t, specifying the set of current
matches, admissions to the queue, and removals from the queue. An outcome process
is then the stochastic process {yt}t≥0 induced by the mechanism’s mapping of histories
to outcomes. In this sense, the outcome process is the realization of the mechanism’s
decisions over time. This, in turn, induces a coarser queue state process, {θt}t≥0. The
state θt ∈Θ := Z represents the number of waiting participants: θt > 0 indicates θt
buyers in the queue; θt < 0 indicates −θt items in the inventory; and θt = 0 is the null
state where the system is empty. A time τ is a null time if the queue is empty, θτ = 0.
7While some of these examples, such as dating apps or retail platforms, incorporate monetary trans-
fers, their role in mediating specific matching and allocation decisions is often limited. For instance,
in dating apps like Tinder, pricing is primarily used to discriminate among users for access to a more
informed pool (e.g., those who have already ’liked’ the user). In contrast, the actual matching follows
the platform’s NTU-based recommendation algorithm.
8That is, the mapping up to time t must be adapted to the filtration Ft.
9While one could formally specify the outcome space and the induced stochastic processes in greater
detail, we omit this formalism here to maintain narrative flow. Furthermore, the methodology of this
survey, which relies on a relaxed program, does not require the full machinery of the general mechanism.
For readers interested in the complete formal specification of dynamic mechanisms and histories in these
contexts, we refer them to the online appendices of Che and Tercieux (forth.) and Che and Choi (2025).
7

Given Markovian arrivals, the system probabilistically restarts after each null time, so
the designer repeatedly faces the same problem. It is therefore without loss to require a
mechanism to depend only on the history since the last null time. Formally, let ω|t be the
history following time t, with time and participant indices reset. A continuation mech-
anism ϕ|t is the outcome process following time t, again with the time and buyer/item
indices reset. A mechanism is regenerative if ϕ|τ(ω) = ϕ(ω|τ) for each null time τ.
Restricting attention to regenerative mechanisms is without loss of generality.
If a mechanism is regenerative, its induced queue-state process {θt} is also regen-
erative.
This class is very general, nesting Markov mechanisms—where decisions
depend only on the current state θt—as a special case. However, the Markov class is
often insufficient. For instance, being oblivious to the arrival order, it cannot imple-
ment standard rules such as First-Come, First-Served (FCFS). Although arrival orders
are payoff-irrelevant in a memoryless process, they can be instrumental for maintaining
dynamic incentives, as we will see.
A remarkable feature of a regenerative process {θt} is that if it is also positive recur-
rent—meaning the expected return time to a null state is finite—it admits a unique sta-
tionary distribution p ∈∆(Z). This distribution describes the system’s long-run behavior,
as empirical time-averages of queue states converge to p almost surely.10 We therefore
focus on a Positive-Recurrent Regenerative Mechanism (PRRM), which induces
such a process. This restriction is without loss, as a non-positive recurrent mechanism
would lead to unbounded queues and infinite expected costs, which is never optimal.11
Incentives.
The incentive issues concerning buyers depend on the set of instruments
employed by the designer, including possible control of entry/exit, information policy, and
10See Asmussen (2003) (p. 170, Theorem 1.2) and Thorisson (1992); we collect a few relevent results
in Section A.
11A process that is not positive recurrent is either transient (queue length diverges) or null recurrent
(expected return to the null state is infinite). Both imply unbounded queue growth and infinite costs,
making them suboptimal. A third possibility is that the queue-state process is positive recurrent but
never reaches the null state. A stationary distribution is still well defined and unique in this case, and
our formulation is valid.
8

the service priority. On both sides (firm and buyers), we require that they at least break
even so that they participate in the mechanism. On the buyer side, the incentive issue
arises because the platform cannot compel a buyer to enter and/or remain in the queue
against their will; therefore, any desired behavior in this regard must be incentivized.
An important issue that arises is the belief a buyer forms about the history leading up
to his arrival, particularly the current queue state. We assume that a buyer forms his
belief about θ ∈Θ based on the stationary distribution p induced by the mechanism. A
justification is that each arriving buyer knows that the mechanism has been operating for
long enough so that the limit distribution applies.12 Note this is when the buyer observes
nothing other than the fact that he “arrives.” If the buyer observes the queue state or
some additional signal about it, he will have a refined belief.
While we deal with this issue more precisely as we get to the specific results, here we
will simply require that a chosen mechanism be incentive compatible: i.e., buyers have
incentives to obey the recommendation associated with ϕ as a Bayes Nash equilibrium.13
We let Φ denote the set of all incentive-compatible PRRMs.
Problem Statement.
We are interested in the following problem:
max
p∈∆(Z) α
"X
k∈N
pk(µv −ck) +
X
ℓ∈N
p−ℓλv
#
+ (1 −α)
"X
k∈N
pkµπ +
X
ℓ∈N
p−ℓ(λπ −ℓd)
#
, [PNTU]
where p = (pi)i∈Z is the stationary distribution induced by some ϕ ∈Φ.
One can
interpret the objective as the long-run time average of the weighted sum of buyer welfare
and profit, where α is the weight for the buyer welfare. I have motivated the relevance of
the program [PNTU] by the use of PRRM. Beyond PRRMs, this program is well-defined
whenever the mechanism the designer employs induces a process on the queue states Θ
12This is established formally in Wolff (1982).
13The agents, buyers in our model, face dynamic environment, so refinements capturing sequential
rationality may be warranted. We use Bayes-Nash equilibria in the formulation of the problem here to
broaden our search for the optimal mechanism. As will become clear, either the equilibrium we study
in each specific case satisfies sequential rationality (as in Section 2.1) or the information policy makes
Bayes-Nash equilibria a relevant concept (as in Section 2.2).
9

that admits a well-defined stationary distribution.
Benchmark: Relaxed problem.
Before jumping into specific results, it is useful to
begin with the first-best benchmark, which ignores the incentive constraints, except for
participation constraints. Namely, consider
max
p∈∆(Z) α
"X
k∈N
pk(µv −ck) +
X
ℓ∈N
p−ℓλv
#
+ (1 −α)
"X
k∈N
pkµπ +
X
ℓ∈N
p−ℓ(λπ −ℓd)
#
, [P′
NTU]
subject to
X
k∈N
pk(µv −ck) +
X
ℓ∈N
p−ℓλv ≥0;
(IRB)
X
k∈N
pkµπ +
X
ℓ∈N
p−ℓ(λπ −ℓd) ≥0;
(IRS)
λpk ≥µpk+1, ∀k ∈N ∪{0};
(Bk)
µp−ℓ≥λp−ℓ−1, ∀ℓ∈N ∪{0}.
(B−ℓ)
The first two constraints, (IRB) and (IRS) are the break-even, or participation, con-
straints for the firm and buyers. The two constraints, (Bk) and (B−ℓ), are necessary for
the stationarity of p. Figure 1 helps explain (Bk). Fix any k ≥0, consider the probability
that the queue state transitions from Θ≤k := {j ∈Z : j ≤k} (to the left of the dotted
line in Figure 1) to Θ>k := Θ \ Θ≤k (to the right of the dotted line) for a brief instant
dt > 0. This probability is at most pkλdt+o(dt)—the probability that there are k buyers
and one more buyer arrives during dt > 0—“at most,” since the new arrival may or may
not join the queue. Likewise, for a brief instant dt > 0, the probability that the queue
state transitions from Θ>k to Θ≤k is at least pk+1µdt + o(dt)—the probability that there
are k + 1 buyers and an item arrives during dt > 0, in which case a buyer leaves with a
good. This is a lower bound, given our assumption of NAD. The latter can’t exceed the
former at a stationary distribution, which explains (Bk).
10

k −1
k
k + 1
k + 2
. . .
. . .
µ
λxk
Figure 1: Balance conditions for a stationary distribution.
An analogous condition explains (B−ℓ).
Any p satisfying (Bk) and (B−ℓ) can be
implemented as a stationary distribution, by probabilistically regulating the entry of
buyers or goods into the queue. Specifically, given (Bk) for k ∈N say, one can choose a
suitably-calibrated admission probability xk ∈(0, 1], so that the balance condition holds
with equality.14
Note that we have reformulated the dynamic mechanism design problem into a static
linear programming (LP) problem in which the designer directly chooses a stationary
distribution p. This methodology holds promise for making a dynamic problem tractable
and is the primary methodological focus of the current survey. We can characterize the
first-best as involving a cutoff structure:
Theorem 1. There exist K∗, L∗∈{0} ∪N ∪{∞} such that the optimal solution p
to [P′
NTU] has support {−L∗, ..., K∗} and satisfies (Bk) with equality for all k ∈N with
k < K −1 and (B−ℓ) with equality for all ℓ∈N with L∗−1. For d sufficiently large,
L∗= 0, and for c sufficiently large, K∗= 0.
Proof Sketch. The interval structure of the support of p is obvious. The support contains
0 since we assume no allocation delays, which makes θ = 0 positive recurrent. To see
that (Bk) and (B−ℓ) are binding for k < K∗−1 and ℓ< L∗−1, write the objective as
X
k
f(k)pk +
X
ℓ
g(ℓ)p−ℓ,
14Strictly speaking, a stationary distribution must satisfy a balance condition between any Θ′ ⊂Θ
and Θ \ Θ′. However, this is implied by the conditions required here.
11

where f(k) := (α+ν)(µv−ck)+(1−α+γ)(µπ) and g(ℓ) := (α+ν)λv+(1−α+γ)(λπ−ℓd),
and ν and γ are (nonnegative) Lagrangian multipliers respectively for (IRB) and (IRS).
Observe that f(·) is strictly decreasing. Hence, if (Bk) is slack for some k < K∗−1,
then one can raise pk slightly and lower pj for j ∈{k + 1, ..., K∗} without violating any
constraints. This modification strictly increases the objective. An analogous argument
works for j = −ℓ, with ℓ< L∗−1. See Che and Tercieux (forth.) for details.
A binding (Bk) means that a buyer arriving at a queue of k buyers must enter the
queue with probability one. Therefore, the optimum in Theorem 1 can be implemented
by a cutoff policy which admits buyers (goods) into the queue only up to some critical
number K∗−1 (resp. L∗−1) and denies entry once K∗(resp. L∗) is reached.15 The
buyers who join the queue are asked to stay until they receive items/services. Of course,
whether and how the buyers may be incentivized to follow these instructions are the key
questions that will be discussed below.
Intuitively, a cutoff policy is optimal since, while storing an additional buyer (resp.
good) has the option value against the future arrival of goods (resp. buyers), the value
of this option diminishes as we store more and more buyers (resp. goods). Consequently,
if it pays to admit a new buyer when there are k buyers in the queue, it does so when
there are j < k buyers in the queue.
2.1
Complete Information Analyses
Naor (1969) pioneered the rational queueing theory, which studies the incentives facing
the agents (buyers in our context) in a queueing environment; see Hassin and Haviv
(2003) for a survey. Much of this literature considers a setting in which buyers have
complete information about the queue state: namely, when they arrive at the queue, they
observe the number of buyers already in the queue. This assumption is less compelling
in many modern settings, such as service call centers, even kidney exchanges, and digital
15Partial rationing may be used for θ = K −1 (resp ℓ= L∗−1) when (IRB) (resp. (IRS)) is binding;
otherwise, there is no rationing.
12

platforms, in which queue information can be, and often is, withheld from the agents.
However, it remains relevant in specific physical queue settings, such as grocery checkouts
and emergency room waiting areas. Below, we assume d = ∞, as is realistic in the service
queue setting, and considered by all authors. At the same time, the results below will
remain qualitatively valid even when d < ∞.
2.1.1
Excessive queueing under FCFS.
Naor (1969) considers the welfare objective with α = 1 and studies the queueing incentive
of the buyers who observe the queue state. The queue discipline is FCFS.
To begin, suppose a buyer is the first to arrive at an empty queue. He can start
receiving service immediately, and it takes 1/µ on average for service to be completed
(since service completion occurs at the Poisson rate of µ). The buyer will join the queue
if and only if v > c/µ.
Suppose next a buyer arrives with k −1 buyers already in the queue. Under FCFS,
his service begins only after all k −1 buyers ahead of him are served, each taking time
distributed exponentially with mean 1/µ. So, his total waiting time (including his service
time) is Erlang-distributed with mean k/µ.16 Hence, he will wish to join the queue if
and only if v ≥ck/µ. Note that once a buyer joins the queue, he can only move up in
the priority order, so his residual mean wait time can only fall, meaning he will want to
remain in the queue until he is served.
It follows that the equilibrium will be of the cutoff structure specified in Theorem 1,
with the maximal queue length given by the marginal buyer (almost) indifferent to joining
the queue, or
KFCFS :=
j
µv
c
k
= max

k : v −ck
µ ≥0

.
The key question is: how does KFCFS compare with the optimal cap K∗?
To answer
this question, recall that we evaluate the welfare in the stationary distribution. Suppose
16A Erlang distribution with (k, µ) is the sum of k independent exponentially distributed random
variables with mean 1/µ: see https://en.wikipedia.org/wiki/Erlang_distribution.
13

that agents queue up to some maximal K. Requiring the balance condition (Bk) for
k = 1, ..., K with equality, we observe
pK
k = λ
µpK
k−1 =
λ
µ
2
pk−2 = ... =
λ
µ
k
pK
0 = ρkpK
0 ,
where ρ := λ/µ is the “load factor.” Using PK
k=0 pK
k = 1, we obtain
pK
k =
ρk
1 + ρ + ... + ρK , ∀k = 0, ..., K.
It is important to observe that pK
k is decreasing in K for each k ≤K; namely, if more
buyers are willing to queue up, the system spends less time on a lower queue state. This
reflects the negative externality conferred by a marginal buyer: a buyer joining the queue
increases the queue length experienced by future buyers and their waiting times.
The
presence of such a congestion externality leads to the following conclusion:
Theorem 2. Assume d = ∞and α = 1. We have K∗≤KFCFS, where the inequality is
often strict, and the queue length under FCFS is larger in first-order stochastic dominance
than that under optimum.
Proof. The second part follows from the first part, since, given K∗≤KFCFS,
k
X
j=0
pKFCFS
j
≤
k
X
j=0
pK∗
j , ∀k.
To prove the first part, recall that we can write the objective as PK
k=0 pK
k f(k), where
f(k) = µv −kc.
(Recall α = 1 and L∗= 0 since d = ∞.)
Now observe that for
all k ≤KFCFS, f(k) ≥0.
Consider any K > KFCFS.
Since KFCFS :=

µ v
c

, f(j) < 0
for each j = KFCFS + 1, ..., K, and pK
k
< pKFCFS
k
for each k ≤KFCFS.
It thus follows
that PK
k=1 pK
k f(k) < PKFCFS
k=1 pKFCFS
k
f(k), whenever K > KFCFS. We thus conclude that
K∗≤KFCFS.
The intuition can be seen clearly in the standard textbook analysis of a market.
14

Figure 2 compares the surplus under FCFS and under the optimum (with α = 1) identified
in Theorem 1, when v = 5, c = 1, and ρ = λ = µ = 1. In each case, the horizontal dotted
line depicts the value realized µv = 5, and the 45 degrees line (dotted) depicts the social
costs associated with waiting, for each state k.
FCFS is depicted in Panel (a), in which KFCFS = v/c = 5, and the social surplus
µv −ck = 5 −k at each state k is simply the difference between the value and the
cost curve. Based on standard textbook reasoning, it may be tempting to conclude that
FCFS is socially optimal, as it exhausts all possible surplus-generating opportunities by
selecting K to be the highest k with nonnegative surplus. This logic, however, overlooks
the “intensive margin”: as noted earlier, with a higher K, each lower inframarginal state
k < K, where the surplus is higher, becomes less likely to occur. By reducing the queue
cap K, welfare may increase due to improvements on the intensive margin.
In the example, with KFCFS = 5, pKFCFS
1
= ... = pKFCFS
5
= 1/6, and the welfare is:
P
k pK
k (µv −ck) = (4 + 3 + 2 + 1)/6 = 5/3. By contrast, under the optimal outcome
with K∗= 2, the likelihood of infra-marginal state increases to pK∗
1
= pK∗
2
= 1/3, and
the welfare rises to (4 + 3)/3 = 7/3. The increased likelihood of states k = 1 and k = 2
under K∗is represented in panel (b) as thicker lines than in (a).
2.1.2
LCFS to the rescue?
Hassin (1985) made an interesting observation that the Last-Come, First-Served (LCFS)
restores the welfare optimal outcome identified in Theorem 1. The idea is as follows.
When LCFS is used, there is always a strict incentive for an arriving buyer to join the
queue, as long as he has the option to leave the queue at a later time, an option we
assume to exist here. Hence, each arriving buyer always joins the queue. The buyer in
the queue who arrived earliest, let’s call him the first incumbent, is last to be served, so
he is most willing to leave the queue if it becomes too long. In other words, the maximal
queue length is effectively chosen in LCFS by the first incumbent’s decision to leave the
queue. The key insight boils down to the following difference between FCFS and LCFS.
15

k
buyer surplus
1
2
3
4
5
v = 5
(a) Surplus under FCFS.
k
buyer surplus
1
2
3
4
5
v = 5
(b) Surplus under K∗= 2.
Figure 2: Comparison of buyer surplus.
Note: v = 5, c = 1, and ρ = λ = µ = 1.
In the FCFS, when a buyer joins the queue when k = KFCFS −1, part of the social cost of
that decision is externalized to the buyers who will arrive later and have to wait longer;17
however, when the first incumbent decides to stay when the new entry triggers the queue
length to increase from K∗to K∗+ 1, nobody else bears that social cost, except for the
first incumbent. This logic suggests that the first incumbent leaves the queue if and only
if a new entry causes the queue length to reach K∗+ 1.
While this logic is compelling, Hassin (1985) offers no formal analysis or proof. I
provide a simple analysis here, whose proof, along with most of the proofs, appear in the
Appendix.
Theorem 3. Assume d = ∞and α = 1. LCFS implements the optimal solution of [P′
NTU]
with KLCFS = K∗.
While insightful and valuable in isolating the source of inefficiency under FCFS, the
practicality of LCFS is dubious. First, the system may be gamed; once they lose their
last arrival status, the agents may leave the queue and reenter it, thereby regaining that
17Such externalities exist even if no new buyer joins the queue when k = KFCFS, since the queue
length eventually becomes strictly below KFCFS and new buyers arrive and have to wait longer because
the marginal buyer decided to join the queue.
16

status and the priority that comes with it. While the designer may manage that problem
by prohibiting reentry, this requires the designer to verify agents’ identities, which the
designer may find costly or undesirable. Second, the analysis and results implicitly assume
“preemption,” meaning that a server may switch from an existing buyer to a new one when
the latter arrives, possibly while serving the former. While this is payoff-irrelevant given
the memoryless property of the exponential service process, in practice, productivity
losses may occur due to service interruptions. While FCFS does not suffer from such
losses, LCFS does. Last but not least, the psychology of queueing is clear that individuals
experience strong resentment when the arrival order is not respected, let alone reversed;
see Larson (1987).
2.1.3
Insufficient queueing under FCFS
Leshno (2022) considers a situation in which the assigned goods are so over-demanded
that agents’ waiting costs do not constitute social costs (provided that, once a good
becomes available, it is assigned to somebody without delay). Namely, in his model, the
total number of people who must wait remains independent of who is served. The goal
is instead to keep them waiting until goods with high match values become available, no
matter how long it may take.
He focuses on the priority rule as an instrument to maximize the incentive to wait
for the matched item. The main tradeoff across different priority rules can be captured
isomorphically within our framework by assuming d = ∞and α = 0.
Just like the
planner in Leshno (2022), our designer with α = 0 wishes to maximize buyers’ incentives
for waiting, to minimize the risk of “buyer stockout” or to maximize the option value of
future matching when goods arrive.
In this case, either K∗= ∞(if µ > λ), or (IRB) is binding at K∗. That is, if K∗< ∞,
then
K∗
X
k=1
pK∗
k (µv −kc) ≈0,
(1)
17

where we use ≈to mean that K∗is the smallest K among those that make the LHS
nonpositive.18 The condition (1) means that
K∗≥µv
c ≥KFCFS,
with the first inequality being strict whenever K∗> 1, in which case FCFS is suboptimal.
Unlike Naor (1969), the problem now is that buyers queue too little, rather than too much.
The difference is due to the objective function: α = 1 versus α = 0.
Leshno (2022)’s main result is that Service in Random Order (SIRO) improves on
FCFS. SIRO assigns the good uniformly at random amongst the buyers in the queue.
Given this rule, a shorter queue is still more profitable to join than a longer queue, so
the entry decision still has a cutoff structure: there exists some KSIRO such that a buyer
enters the queue if and only if k < KSIRO and those who enter the queue stay until they
receive the goods.
Theorem 4. Assume d = ∞and α = 0. Then, KFCFS ≤KSIRO ≤K∗, and SIRO attains a
weakly higher (resp. lower) value of the objective for the designer than does FCFS (resp.
the optimum).
The intuition behind Theorem 4 is made clear via the following example (adapted
from Che and Tercieux (forth.)). Assume µ = λ = 1 and 3
2c ≤v < 2c. The former
inequality implies that K∗= 2, meaning the optimal cutoff policy accommodates up to
two buyers in the queue,19 and the implication of the latter condition will become clear.
Figure 3 plots the expected wait costs for a buyer who arrives at an empty queue
(k = 0) and a queue with one buyer (k = 1) under FCFS. Under FCFS, a buyer will
expect to wait 1/µ = 1 unit of time for the good if k = 0 and 2/µ = 2 units of time if
k = 1. The associated waiting costs are c and 2c, respectively. If v < 2c, as described in
18More precisely, the rationing of entry in state k = K∗−1 is calibrated to satisfy the equality
exactly. The chosen K∗with the rationing will be precisely the smallest K among those that make the
LHS nonpositive.
19Little’s law can be used to show that the ex ante expected wait time conditional on there being a
buyer for queue with K = 2 equals:
p2
1+2p2
2
λ(p2
0+p2
1) = 3/2, since p2
0 = p2
1 = p2
2 = 1/3 given µ = λ = 1.
18

the figure, then a second buyer will refuse to join the queue, so FCFS supports at most
one buyer in the queue.
k
Waiting costs
2c
c
v
k = 0
k = 1
(a) SIRO supports 2 agents in the queue;
FCFS can’t.
k
Waiting costs
2c
c
v
k = 0
k = 1
(b) SIRO can’t support 2 agents.
Figure 3: Comparison of queueing rules under full information
Notes: The black and red dots represent the expected waiting cost under FCFS and SIRO, respectively.
Note that a buyer arriving in state k = 0 enjoys a strictly positive surplus. If one
can transfer this “slack” incentive to a buyer arriving in state k = 1, the latter may
be incentivized to join the queue. This is precisely what SIRO does by “flattening” the
waiting cost curve. By randomizing service priority, SIRO increases the priority of a buyer
arriving in state k = 1 at the expense of the buyer arriving in state k = 0, compared to
FCFS.
Observe that the waiting cost curve is not completely flat even under SIRO.20 If v is
sufficiently high (panel (a)), SIRO can restore the optimum. However, if v is lower (panel
(b)), in particular if v < 5
3c, SIRO can’t support two buyers in the queue; see panel (b)
of Figure 3.
It is possible to completely flatten the waiting cost curve by transferring priority to
the second arrival even further—under the rule called the Load Independent Expected
Waiting (LIEW) by Leshno (2019)). Such a rule maximizes the queueing incentive by
20The mean waiting times are 4/3 if k = 0 and 5/3 if k = 1; see the equations in the proof of
Theorem 4.
19

equalizing the wait times for a buyer joining in state k = 0 versus joining in state k = 1.
In the example, LIEW supports two agents in the queue, provided that v is no less than
1.5c, the conditional mean waiting cost for a queue with K = 2. That is, LIEW can
attain the optimal solution of [P′
NTU] (with α = 0).
In summary, FCFS is suboptimal and outperformed by rules such as SIRO and LIEW
when the queue is fully observable. However, if the designer can control information,
the queueing rule becomes irrelevant for the incentive to join the queue. Suppose the
designer informs buyers only whether k ∈{0, 1} (“recommend to join”) or not. Then,
a buyer can’t distinguish between k = 0 and k = 1 upon his arrival; he then forms the
identical belief between k = 0 and k = 1, thus facing the expected waiting time of 1.5,
just like LIEW. In other words, the information policy can replace the LIEW. Under this
(optimal) information policy, the mean wait time—and hence the incentive to join the
queue—is the same across all queueing rules, so they cannot be differentiated on this
account.
Meanwhile, LIEW may create pathological incentives for buyers once they join the
queue.
Just like LCFS, after joining the queue, a buyer realizes that his remaining
expected wait time increases as time passes.
This is because LIEW, to equalize the
wait time between a buyer arriving in state k = 0 and a buyer arriving in state k = 1,
must worsen the former buyer’s priority when a second buyer arrives, putting him in
a position akin to one who arrives in k = 1 under FCFS. This means that the buyer
will be tempted to exit the queue once a second buyer joins and takes away his priority.
Consequently, LIEW (and any other rules) can’t implement the optimal outcome under
complete information.
2.2
Information Design and the Optimality of FCFS
While the complete information assumption is realistic for some classic contexts, modern
businesses, particularly platforms, do have substantial control over what information
buyers can or can’t have. Call centers, a quintessential example of queue systems, often
20

leave customers without precise information about how long they have to wait. Public
housing systems, a motivating example in Leshno (2022), often keep the eligible recipients
in the dark about their spots in the waitlists. Digital platforms, which engage customers
through mobile or web interfaces, have even more control over information. Ride-hailing
platforms, such as Uber or Lyft, inform customers about the locations of matched drivers
but do not reveal other potential matches or allow them to choose from these options.
Information design confers the designer additional power to control buyers’ incentives.
Recall that under complete information, FCFS offers excessive incentives for queueing
from the consumer surplus standpoint (α = 1) but insufficient incentives from the pro-
ducer surplus standpoint (α = 0). As we already hinted in the last example, information
design can help overcome the latter problem. But without additional tools, information
design alone cannot implement the optimal solution [P′
NTU] incentive-compatibly.
Lingenbrink and Iyer (2019) and Anunrojwong, Iyer, and Manshadi (2020) studied
information design by the designer who seeks to maximize consumer welfare (i.e., α = 1)
under FCFS. They showed that the optimal information involves two signals; either the
current queue state is “k < K” or “k = K,” for some K ∈N, with the former signal
intended for encouraging buyers to join the queue and the latter for discouraging buyers
from doing so.
This result discovers the well-known “folk” theorem in economics: a
mechanism designer should reveal no more than the actions that she recommends to an
agent, the simple logic being that any distinct signals leading to the same action can
be pooled into one without violating any incentive constraints.21 Without admissions
control, however, this policy itself cannot keep buyers from joining the queue at the
signal “k = K” unless K ≥KFCFS. Anunrojwong, Iyer, and Manshadi (2020) shows that
a cap ˆK < KFCFS (where ˆK may possibly equal K∗) can be implemented if there is an
additional type of buyers who incur no wait costs (or have a very low outside option) and
are willing to join the queue, regardless of the current queue length. Then, the signal
“k ≥ˆK” would mean that there are many more than KFCFS buyers already in the queue,
21Since the desired action is incentive compatible under each signal, it must be on average over the
multiple signals, and hence under the pooled signal.
21

making it incentive-compatible for a buyer with cost c > 0 not to join when learning
“k ≥ˆK.”
This problem does not arise when the designer can control admissions into a queue.
Formally, the designer can control admission by refusing to serve buyers whom the de-
signer wishes to keep out. In practice, a call center discouraging entry (accompanied by a
repetitive soundtrack) is often effective for this purpose. Che and Tercieux (forth.) con-
sider the designer who controls three sets of instruments: (i) admission control (buyers
can be denied entry into a queue or even removed from it); (ii) service priority allocation
(or queue disciplines); and (iii) information design. They consider buyers’ incentives not
only to join the queue but also to stay in the queue, when recommended by the mecha-
nism. The literature has largely ignored this latter incentive issue.22 Che and Tercieux
(forth.) obtain the following result:
Theorem 5. The optimal solution of [P′
NTU] is implementable by FCFS under the optimal
information.
Theorem 5 states that the optimal solution to the relaxed program [P′
NTU] which ignores
the incentives problems can be implemented by an admissions control which keeps the
buyers from the queue whenever K∗is reached (with possible rationing at k = K∗−1),
an information policy which imply issues recommendation to join the queue if k < K∗
(subject to possible rationing at k = K∗−1), and a priority rule of FCFS.
For a sufficiently large α ∈[0, 1], the problem is the excessive queueing a la Naor
(1969); this problem is solved simply by keeping buyers from joining to queue beyond
K∗.23 The difficult situation is when α is so low that buyer have insufficient incentives for
queueing, as was seen earlier. We already saw that, given (IRB), the optimal information
policy provides buyers with sufficient incentives to join the queue, regardless of the priority
22The exception is Hassin (1985), where the first incumbent’s decision to leave the queue plays a
crucial role in sustaining welfare maximum.
23Equivalently, as noted below, the optimum K∗can be also implemented by a lottery of the real
good (with value v) and a null good, calibrated to induce K∗. If a buyer rejects a null good, he loses his
priority.
22

rule. However, it is not easy to provide them with incentives to stay in the queue once
they join, which is what the optimal cutoff policy calls for.
To illustrate, consider the earlier example in which d = ∞, v = 1.5c, and µ = λ = 1.
We note that K∗= 2, at which (IRB) is binding. Under optimal information, any arriving
buyer invited to the queue expects to wait for 1.5 units of time, assuming he will never
leave the queue. Given (IRB), this wait time incentivizes buyers to join the queue as long
as k < K∗= 2 under any of the priority rules; see Section D for the precise argument.
However, the incentive to stay in the queue once a buyer joins may or may not hold,
depending on the priority rule. Figure 4 plots the mean residual wait times for a buyer
who has spent time t ≥0 on the queue. We consider five standard rules: FCFS, SIRO,
LIEW, LCFS, and LCFS-PR, where LCFS-PR is the LCFS with “preemption,” namely,
a rule in which an old agent leaves when a new agent enters the queue.
Figure 4: Expected wait times under alternative queueing rules.
Note first that the mean wait time is 1.5 at t = 0, as observed below, for all priority
rules. Since v = 1.5c, buyers are thus indifferent to joining the queue (assuming they
will stay until they are served). However, once they have joined, the mean residual wait
times diverge under alternative rules as one’s time on the queue t rises: it decreases under
FCFS but increases under all other queueing rules. Hence, the agents will have incentives
to stay until they are served under FCFS, but they will abandon the queue unless served
23

immediately under every other rule. This difference means that the FCFS outperforms
the other queueing rules.24
Under FCFS, a buyer’s service priority increases over time as earlier arrivals leave the
queue. Hence, conditional on the initial queue length, one expects to wait less over time,
so the mean residual wait time decreases gradually. However, there is also a countervailing
force. Since an agent is not told about whether k = 0 or k = 1 upon joining the queue,
his belief about the initial queue length will also be updated as time progresses. On
this account, he becomes pessimistic over time since the fact that he is still in the queue
indicates that he likely underestimated the initial length of the queue when he joined
it. It turns out that the good news effect dominates the bad news effect in the M/M/1
model and even more broadly (see Che and Tercieux (forth.)).
The mean residual wait time increases in other rules since the “seniority” in arrival
order does not carry as much priority as in FCFS. This is most evident with LCFS and
LIEW. As time passes, one faces a new entrant who takes away all or part of his priority,
and one’s residual wait time increases as time passes. SIRO suffers the same issue, albeit
to a lesser degree. Under SIRO, one’s priority does not improve over time; only the queue
length and one’s belief about it matter. As time passes, one becomes more pessimistic on
this account as he believes more buyers are in the queue than he initially thought, thus
increasing his remaining wait time.
At a more fundamental level, the above difference in queueing rules can be traced to
the fact that they entail different distributions of wait times, although their mean is the
same. In particular, the wait time distribution is most “fair,” or least dispersed, under
FCFS among all queueing rules; see Shanthikumar and Sumita (1987) for establishing this
result under M/M/1 setup. This means that both unusually short waits and unusually
long waits are rare under FCFS compared with other rules.
24The figure implies that under each of the other queueing rules, it cannot be an equilibrium that
agents will join the queue up to two and stay until they are served. It is unclear what will happen in
equilibrium; they may randomize in leaving the queue, and/or they may excessively enter with a plan to
leave soon after. Regardless, Che and Tercieux (forth.) shows the designer will be worse off than under
FCFS for some values of λ.
24

This is easily seen in any realized within-cycle sample path of arrival times and depar-
ture times. Figure 5 illustrates an arbitrary sample path.25 A priority rule corresponds to
a bipartite matching between arrival times and departure times. In the M/M/1 model,
the buyer who arrives first departs first under FCFS; this means that no two edges in
the corresponding bipartite matching “cross” each other; see panel (a). By contrast, in
any rule differing from FCFS, such as LCFS or SIRO, the corresponding matching has
crossing edges, as depicted in the panel (b). This difference means a lower dispersion in
wait times under FCFS; in the example, the wait times are 1.5, 1.5, and 2 in (a), whereas
they are 0.5, 0.5, and 4 in (b), with the same mean 5/3.26
How does the wait time distribution affect a buyer’s belief about residual wait time?
To study this, it is convenient to construct a probability space, as well as a buyer’s belief,
in two steps: (i) a sample path of arrival and departure times is first drawn according to
exponential distributions (e.g., Figure 5) and (ii) a buyer’s arrival time is then assigned
to one of the arrival times (e.g., across a1, a2 or a3).27 One can then “couple” the sample
paths under two queueing rules, such as (a) and (b).
arrival times
departure times
a1
1
a2
2
a3
3.5
d1
2.5
d2
4
d3
5
(a) FCFS rule.
arrival times
departure times
a1
1
a2
2
a3
3.5
d1
2.5
d2
4
d3
5
(b) LCFS or SIRO.
Figure 5: Comparison of queueing disciplines in a sample path.
Recall we are considering an optimal information policy, so a buyer does not observe
the queue state when he joins the queue and thereafter. Suppose a buyer forms his belief
25Any within-cycle sample path must have the same number of arrivals and departures.
26In general, since the wait time distribution becomes mean-preservingly contracted whenever any
pair of crossed edges are swapped to reduce crossing, and since one can always repeat this swapping
operation to move from any arbitrary matching to the matching under FCFS, it follows that the wait
times under FCFS are a mean-preserving contraction of the wait times under any queueing rule.
27The “calendar” time of one’s arrival or departure carries no information in the steady state.
25

on (ii) for each arbitrary sample path (i) he considers possible, e.g., a path like the one
portrayed in the example.28
Having spent time t < 0.5 in the queue, the remaining
expected wait time is (5/3)−t for both (a) and (b), conditional on the illustrated sample
path. But after spending time t ∈[0.5, 1.5), the remaining wait times diverge under two
rules: it is (5/3) −t in (a), whereas it is 4 −t in (b), as the buyer infers his arrival time
to be a1.29
Intuitively, dispersed waiting times make one pessimistic over one’s residual wait
times, worsening one’s dynamic incentives. As time passes, the fact that one still remains
in the queue indicates that he has “missed the early breaks” and, therefore, the residual
wait will be longer. The fairness property of FCFS alleviates this problem, enabling the
designer to implement the optimal outcome.
2.3
Large Market Limit
Suppose the market becomes dense as λ, µ →∞with the balance parameter ρ = λ/µ
held constant. The limit of such markets is captured by a “fluid” model with a unit mass
of items and a mass ρ = λ/µ of buyers arriving at every instant.30
In such large markets, the stochasticity of the arrival processes and any uncertainty
facing the designer disappear.
Hence, the fundamental reason for holding queues of
buyers or items no longer exists. Yet, the incentive problem recognized by Naor (1969)
manifests itself extremely. Recall that, given complete information, no admissions control,
and FCFS, buyers will queue up to a level KFCFS =
 v
cµ

.
Let’s focus on the interesting case of ρ ≥1.31
In this case, there is a perennial
28The explanation below conditions on each sample path. In the original problem, a buyer does not
observe the realized sample path, so he also updates his belief on the sample path based on the elapsed
time. In this sense, the explanation is intended to provide intuition on the force at work rather than a
precise proof, which explicitly studies the evolution of beliefs based on one’s queue position; see Section D
for details.
29For t ∈[1.5, 3], then the remaining wait time is 3 −t in (a), and it is 4 −t.
30The term ’fluid model’ (or ’fluid limit’) is standard in queueing theory, operations research, and
the study of stochastic networks to describe a deterministic approximation of a system where discrete
arrivals and departures are smoothed into a continuous flow.
31If ρ < 1, this maximal queue length is never binding, as λ, µ →∞, the excess supply absorbs
demands with probability one, and the wait time shrinks to zero in probability.
26

excess demand, and all buyers queue up to the KFCFS level. In the limit, the normalized
average queue length, normalized by µ, and each buyer’s wait time, converge to ρv
c and
v
c, respectively, in probability. In short, all buyers queue up excessively to a degree that
leaves them with no surplus.
Theorem 5 remains valid even in this large market setting. Yet, the queueing rule/priority
rule becomes irrelevant since the distribution of waiting time becomes degenerate. Re-
calling ρ ≥1, one can ensure that virtually all goods will be allocated to buyers with
vanishing delays in the limit, by setting K∗= (1 −ϵ)µ, for an arbitrarily small ϵ > 0.
The idea is that the small ϵ here creates states of excess supply and eliminates delay for
those admitted into the queue. In particular, in the fluid model, we have the following.
Theorem 6. Consider a continuum economy in which a unit mass of goods and a mass
ρ ≥1 of buyers arrive at each instant.
(i) Given the setup of Naor (1969) with complete information, FCFS, and no admis-
sions control, all buyers wait for v/c time to be served. The (per-unit time) total
surplus is 0.
(ii) An optimal mechanism allocates (virtually) all goods to a queue of buyers whose
length is capped so that no delay occurs.
Instead of a binding queue cap, the optimal policy (ii) can also be implemented by
assigning a lottery that awards a unit of good with probability (1/ρ) −ϵ, under FCFS
with complete information.
2.4
Screening buyers with heterogeneous values
The large market limit introduced above provides a convenient segue to study how wait-
list/queueing can be used to allocate goods to buyers with heterogeneous values. Ashlagi,
Monachou, and Nikzad (2025), Arnosti and Shi (2020), and Castro, Ma, Nazerzadeh, and
Yan (2021) study a fluid (or large market model) model in which the designer allocates
27

items with heterogeneous qualities to buyers with different values via some form of waitlist
policies.32
Here, I present a simplified version of Ashlagi, Monachou, and Nikzad (2025) to
present the main insights from these papers. Consider a fluid model in which a unit mass
of items and a mass ρ = λ/µ > 1 of buyers arrive at every instant. For the current
purpose, it is convenient to interpret the model alternatively so that a mass ρ of items
of heterogeneous qualities arrive at each instant, out of which mass 1 of items has high
quality equal to one, and a mass ρ −1 has low quality equal to zero. The real new
feature is that the buyers have heterogeneous values v of the item, distributed from [0, 1]
according to a CDF F, which admits a strictly positive density f on (0, 1). Assume that
the inverse hazard rate 1−F(v)
f(v)
is decreasing in v for all v ∈(0, 1).
Ashlagi, Monachou, and Nikzad (2025) considers discrete time (as opposed to the
continuous time considered here) and allows for finite quality levels, rather than the two
levels, 0 and 1, as in the current model. Nevertheless, the current model captures the
central economic insights of the paper. The benefit is that we maintain the workhorse
framework presented in this survey.33 The designer’s objective could be social welfare
or “allocative efficiency,” which accounts for the gross surplus, ignoring the buyers’ wait
costs.
Without loss, the designer specifies the eventual allocation probability X(v) and the
expected wait time W(v), for the buyer who has just arrived and reported value v at each
32Mekonnen (2019) develops a closely-related model with two-sided matching with frictional search.
The trade-off between random search and directed search, the focus of this work, mirrors the trade-off
between screening and random allocation discussed here. Su and Zenios (2004) and Schummer (2021)
study a related issue in non-fluid models, but without the type of heterogeneity considered here. Indeed,
I am not aware of any NTU analysis that allows for value heterogeneity in the canonical queueing (i.e.,
non-fluid) setup, and this remains an open area of research. Even though Leshno (2022) and Baccara,
Lee, and Yariv (2020) have two types of agents and items, the analysis is virtually isomorphic to a
one-type model. Che and Choi (2025) studies a fully general model of the value heterogeneity, but in a
TU setup.
33The multiple quality version required the authors to express feasible allocation—defined as a map-
ping from buyer values to the expected quality—as a mean-preserving contraction of the most positively
assortative matching allocation. They then use Kleiner, Moldovanu, and Strack (2021)’s characterization
of extreme points of such (majorized) allocations to argue that the optimal solution takes the form of a
certain ironed version of the most positively assortative matching. The current two-level quality model
simplifies ironing.
28

instant. Since these are equilibrium objects, the pair must satisfy:
U(v) := vX(v) −cW(v) ≥0, ∀v;
(IRNTU)
U(v) ≥vX(v′) −cW(v′), ∀v, v′;
(ICNTU)
These specifications of (IRNTU) and (ICNTU) already portend the possible role of “waiting
costs” as a screening device, often reserved for monetary transfers in the TU model. This
analogy is not accidental; the wait costs will play the same role as transfers, except for
one important difference: the wait costs entail welfare costs whereas monetary transfers
would not.
As usual, one can use the standard envelope characterization to replace (ICNTU) and
(IRNTU) by:
U(v) =
Z v
0
X(s)ds ∀v;
(ENTU)
X(·) is nondecreasing.
(MNTU)
Next, let p(v) denote the steady-state mass of buyers with values above v in the queue.
(This corresponds to the stationary distribution in our framework, even though p(0), the
total mass of buyers in the queue, need not equal 1.) Then, we must have
ρ
Z 1
v
W(s)f(s)ds = p(v), ∀v;
(LNTU)
ρ
Z 1
0
X(s)f(s)ds ≤1.
(RFNTU)
The condition (LNTU) follows from Little’s law, which, for each v, relates the total wait
time for incoming buyers with values above v to the average queue length of these types,
which in our continuum economy collapses degenerately to p(v), the total mass of these
29

types in the queue. The condition (RFNTU) ensures that the mass of promised allocation
(after possible wait) equals the mass of items available at each instant. (Recall that with
our normalization, the normalized rate at which the items are received equals one.) This
condition constitutes a Border style reduced-form auction characterization; see Che and
Choi (2025) as well as the next section for further details.
Ashlagi, Monachou, and Nikzad (2025) consider allocative efficiency as an objective,
in which case the problem is:
max
X,W,p ρ
Z 1
0
vX(v)f(v)dv
[P′′
NTU]
subject to
(IRNTU), (ICNTU), (LNTU), and (RFNTU),
Theorem 7. Assume ρ > 1. At the optimal mechanism solving [P′′
NTU], all buyers with
value v ≥˜v wait for ˜W := ˜v/c amount of time, where ρ[1 −F(˜v)] = 1. More formally,
X(v) = 1{v≥˜v}, W(v) = 1{v≥˜v} ˜W, and p(v) := ρ ˜v
c[1 −F(max{v, ˜v})].
Proof. Consider a further relaxed problem in which (MNTU) and (LNTU) are absent. Con-
sider the Lagrangian (ignoring the constraint X(·) ∈[0, 1]):
L =
Z 1
0
(v −ζ)X(v)f(v)dv,
where ζ ≥0 is the multiplier for (RFNTU). Letting ˜v := inf{v : v ≥ζ}, we must have
X(v) = 1{v≥˜v}, which satisfies (MNTU). If ζ = 0, then X(v) = 1 for all v, which violates
(RFNTU) since ρ > 1, so (RFNTU) is binding and ˜v is pinned down by ρ[1 −F(˜v)] = 1. By
setting W(v) = 1{v≥˜v}˜v/c, (ENTU) and (LNTU) are satisfied. The chosen set of solutions
so far is feasible and satisfies the complementary slackness condition. So, by the weak
duality, it is an optimal solution to [P′′
NTU].
Allocative efficiency requires allocating goods to buyers with values above a market-
clearing “price” ˜v. In the NTU context, the price can only be paid in waiting costs,
requiring a wait time of ˜W := ˜v/c, which is supported by the steady-state queue length
30

of ρ[1 −F(˜v)]˜v/c. Facing this queue length and the requisite wait time, buyers enter if
and only if v ≥˜v. Note that the expected waiting time is pinned down by Little’s law,
regardless of the queueing discipline. One simple rule could be FCFS;34 the allocation
can be seen as heterogeneous-value version of Naor (1969), or Theorem 6-(i).
Panel (a) of Figure 6 depicts the allocatively efficient outcome when ρ = 1.5 and F is
uniform. Under the alternative interpretation with masses 1 and ρ−1 of high-quality and
zero-quality goods, the allocatively efficient outcome can be implemented by an FCFS
with a deferral right: namely, buyers offered zero-quality goods can refuse assignment
without losing their spots.
It is important to recognize allocative efficiency doesn’t correspond to social welfare
maximization—an important difference relative to the TU setup that will follow. Ash-
lagi, Monachou, and Nikzad (2025) also considers social welfare maximization. Here, we
consider an objective similar to the one considered in the earlier section:
max
X,W,p αρ
Z 1
0
U(v)f(v)dv + (1 −α)ρ
Z 1
0
πX(v)f(v)dv
[P′′′
NTU]
subject to
(IRNTU), (ICNTU), (LNTU), and (RFNTU),
where α ∈[0, 1] and 1 −α are respectively the weights the designer assigns to buyer
welfare and firm profit (π is generated whenever a buyer is assigned/served).
Substituting (ENTU) into the objective function and simplifying the objective function
allows us to rewrite the problem as:
max
X,W,p ρ
Z 1
0
K(v)X(v)f(v)dv
subject to
(RFNTU), and (MNTU),
where K(v) := α 1−F(v)
f(v)
+ (1 −α)π. Given our assumption of decreasing inverse hazard
34Other priority rules also work in the fluid model, since the residual mean wait time is degenerate at
˜v/c, regardless of the queueing rule.
31

rate, K(·) is nonincreasing for all α.
Theorem 8. Assume ρ > 1. At the optimal mechanism solving [P′′′
NTU], all buyers are
served with a lottery X(·) ≡1/ρ without any delays (i.e., W(v) = p(v) ≡0).
Proof. Fix any nondecreasing X(·) satisfying (RFNTU). Let ¯x :=
R 1
0 X(v)f(v)dv ≤1/ρ.
Then, since K(·) is nonincreasing and nonnegative, by the Cauchy-Schwarz inequality,
Z 1
0
K(v)X(v)f(v)dv ≤
Z 1
0
K(v)f(v)dv

¯x ≤
Z 1
0
K(v)f(v)dv
 1
ρ,
where the last inequality follows from (RFNTU).
Allocative efficiency corresponds to social welfare maximization in the TU setting;
however, this is not the case in the NTU setup, since the price paid by buyers to claim
objects is not a transfer but a wasteful social cost. The dark blue area in Panel (a),
therefore, constitutes the only net social surplus; the red area is dissipated through waiting
costs.
When the buyers are homogeneous, such waste can be eliminated through admissions
control or a lottery, as shown in Theorem 6-(ii). However, doing so is costly here since the
designer must sacrifice allocative efficiency. In principle, it is unclear how the designer
would trade off these two objectives. Yet, given the declining inverse-hazard rate condi-
tion, the social optimum (α = 1) completely sacrifices allocative efficiency to minimize
waiting costs. In other words, complete pooling with random allocation is prescribed,
which entails no delay. In the example, the welfare under full screening is 2/9 (the dark
blue area in (a)), whereas the welfare under pooling is 1/2 (the light blue area in (b)
times 1/1.5). Under the alternative interpretation with a mass ρ −1 of zero quality good
and mass 1 of high quality good, this complete pooling can be implemented with an
FCFS without a deferral right; namely, buyers offered zero quality goods can’t refuse the
assignment. See Arnosti and Shi (2020) and Castro, Ma, Nazerzadeh, and Yan (2021)
for proposing queuing mechanisms with a similar feature.
32

value
1
1
1.5
˜v
(a) Allocative efficiency (α = 0).
value
1
1
1.5
˜v
(b) Welfare optimum (α = 1).
Figure 6: Comparison of welfare measures.
The main insight of Theorem 8 is traced back in its intellectual provenance to McAfee
and McMillan (1992), who showed that a bidding ring seeking to maximize its members’
welfare would prefer to assign a winner at random when a knock-out action is infeasible (so
transfers can’t be used), given the same inverse hazard rate condition,35 and to Condorelli
(2012) and Hartline and Roughgarden (2008), who used the same logic to argue that
complete pooling is socially optimal when the designer must rely on costly signaling to
screen agents. The difference here is that costly signaling must take the form of waiting
in a queue, which, in the steady state, must require the queue length to be adjusted
endogenously to satisfy incentive compatibility. The current overview also reveals the
welfare cost of the queue as a manifestation of Naor (1969)’s problem and pooling as its
remedy.
3
Transferable Utility Model
We now turn to the Transferable Utility (TU) model. While the previous section focused
on markets where prices play a limited role in allocation, many dynamic environments
utilize monetary transfers to balance competition and manage supply and demand. Fol-
lowing the framework established in Che and Choi (2025), we generalize the classic mech-
anism design paradigm of Myerson (1981) to the dynamic setting with asynchronous and
35See also Che, Condorelli, and Kim (2018) for the optimality of pooling in the auction environment,
for the same reason.
33

stochastic arrivals of both items and buyers.
3.1
Setup
The model is the same as before in its basic primitives: in a continuous time t ≥0, the
designer(platform) receives units of a homogeneous good arriving at a Poisson rate of
µ > 0. Buyers with unit demand arrive at a Poisson rate of λ > 0. What is different,
however, and similar to Myerson (1981), is that buyers have heterogeneous values for the
good. Specifically, each buyer has value v ∈[0, 1], drawn independently for each buyer
at the time of his arrival according to a CDF F with a density f that is strictly positive
and absolutely continuous. We assume that the virtual value J(v) = v −(1 −2α)1−F(v)
f(v)
is nondecreasing in v for all α.36 Each buyer is privately informed of his value v.
As before, the designer may store buyers in a queue, at c > 0 per unit time per buyer,
if there are no items available, and she can store goods in inventory at cost d > 0 per
unit time per item, in case there are no buyers.
Unlike the NTU model, buyers have transferable utilities that are linear in money.
So, if a buyer with value v spends time t in the queue, makes a monetary payment of y,
and receives the good, his payoff is v −y −ct. If he spends time t in the queue and pays
y but does not receive the good, his payoff is −y −ct.
Mechanisms.
We again focus on regenerative mechanisms, as defined in the NTU
model. The TU setting introduces two key differences. First, buyers have heterogeneous
private values v ∈[0, 1]. By the (Bayes-Nash) revelation principle, we can restrict our
attention to direct mechanisms ϕ that condition on buyers’ reported values. Second, the
mechanism’s set of outcomes is expanded to include monetary payments. Therefore, a
mechanism ϕ is a non-anticipatory, measurable mapping from histories (which include
36This condition holds when F is uniform for all α ∈[0, 1]. More generally, for distributions with a
non-increasing inverse hazard rate, the condition is always satisfied for α ∈[0, 1/2]. For α > 1/2, the
condition requires that the inverse hazard rate does not decrease too rapidly; specifically, we require
1 −(1 −2α) d
dv[ 1−F (v)
f(v) ] ≥0. If this strong form of regularity is violated, then I conjecture that a form of
ironing suggested by Myerson (1981) will apply.
34

primitive arrivals and all reported values) to outcomes (queuing decisions, matches, and
payments).
This richer setting necessitates a more complex state space. A queue state θ must
now capture not only the queue length k ∈Z (where k < 0 denotes item inventory) but
also the reported values of buyers currently waiting. We represent this by an ordered
vector v = (v1, v2, ....) ∈[0, 1]N of reported values. Let V ⊂[0, 1]N be the set of all such
ordered vectors. We continue to impose the No Allocation Delay (NAD) condition. The
full queue state space is thus Θ := (∪l∈N{−l}) ∪{0} ∪V.
As before, we restrict attention to Positive-Recurrent Regenerative Mecha-
nisms (PRRMs). Any such mechanism ϕ induces a positive recurrent process on the
state space Θ, which admits a unique stationary distribution p ∈∆(Θ). We require the
mechanism to be incentive-compatible. Let Φ denote the set of all PRRMs in which
buyers have incentives to follow all recommendations, including reporting their values
truthfully, as a Bayes-Nash equilibrium. This equilibrium is evaluated assuming an in-
coming buyer’s prior on the queue state is given by the stationary distribution p induced
by the mechanism.
The problem facing the designer is:
sup
ϕ∈Φ
Z
θ
Z 1
0
h
αλU M(v; θ) + (1 −α)λT M(v; θ) −
X
ℓ∈N
1{θ=−ℓ}ℓd
i
f(v)dvp(dθ),
[PTU]
where α is the weight on the buyer welfare, U M(v; θ) and T M(v; θ) are respectively the
expected utility and payment for a buyer with value v arriving in state θ under mechanism
ϕ. The problem specializes to revenue maximization when α = 0 and to the welfare
maximization, or equivalently allocative efficiency, when α = 1/2.
Relation with Literature.
This model generalizes the static mechanism design frame-
work, as presented in Myerson (1981), by considering a dynamic environment, which is
more descriptive of platform-mediated modern marketplaces. In such a setting, the key
role of the designer is not just to allocate items to buyers when they are all present
35

simultaneously, but also to manage and transfer competition across time by judiciously
storing buyers or goods.
Importantly, however, this model does not nest the NTU model surveyed earlier.
Transferability makes it easy for the designer to control buyers’ incentives. For example,
dynamic incentives can be managed simply by reimbursing buyers for the waiting costs
they incur based on the amount of time they spend in the queue. Recall that most of the
analytical challenge in the previous section resulted from the difficulty associated with
managing buyers’ incentives to queue in the NTU model; this issue becomes easier to
handle in the TU setup.
3.2
Optimal Mechanism
The problem [PTU] is difficult to solve. Instead, a relaxed problem is set up as follows.
Fix any mechanism M ∈M∗∗. It induces an interim allocation probability X(v) and
payment T(v) for a buyer who has just arrived and reported value v, where the payment
is made after (or net of) the reimbursement of waiting costs, which we assume in the
sequel.37 Since M is incentive compatible, they must satisfy:.38
U(v) := vX(v) −T(v) ≥0, ∀v,
(IR)
U(v) ≥vX(v′) −T(v′), ∀v, v′.
(IC)
Obviously, not all X(v) ∈[0, 1] is feasible. The allocation promised to a buyer must be
compatible with the stochastic supply of goods as well as with the promises the designer
makes to buyers who arrived before and those who will arrive in the future. To handle
the feasibility issue, we first let pk(v) denote the probability that exactly k buyers have
values strictly above v in the steady state. Accordingly, pk(0) denotes the probability that
there are exactly k buyers in the steady state queue. Also, as before, let p−ℓ, ℓ∈N
37Recall we already observed that the designer may, without loss, reimburse waiting costs.
38Note that (IR) ignores possible double deviations, so it is necessary but not sufficient for buyers to
report truthfully. This is not a problem since we are considering a relaxed program.
36

denote the stationary probability that there are ℓitems in the inventory.
Then, feasibility of X(·) requires:
λ
Z 1
v
X(s)f(s)ds ≤µ
∞
X
k=1
yk(v)pk(v) +
∞
X
ℓ=1
p−ℓ
Z 1
v
zℓ(v)f(v)dv,
∀v ∈V,
(RF)
where yk(v) is the probability that an incoming good is allocated to one of the k buyers
with values above v and zℓ(v) is the probability with which an incoming buyer with
value v is allocated the good when there are ℓitems in storage. The LHS describes total
allocation promises made to types above v per unit of time, while the RHS describes
total allocation made to the buyers with values above v per unit time, noting that an
allocation occurs when a good arrives with buyers waiting in the queue or when a buyer
arrives with goods in storage. This condition is similar in spirit to Border (1991) and
Che, Kim, and Mierendorff (2013) but has an added temporal dimension.
We next turn to the feasibility of the distribution p = (pk). Since these are stationary
objects, they must respect balance conditions. For a subset Z≤−ℓ⊂Θ, for θ = −ℓ, for
some ℓ∈N, we must have a balance condition between transitions between Z≤−ℓand
Θ \ Z≤−ℓ:
µp−ℓ≥λp−(ℓ+1),
(Bℓ)
somewhat analogously to the condition we had in the NTU setup. What is new and
potentially difficult is the balance condition for each measurable set V′ ⊂V ⊂Θ, namely
the set of profiles of values reported by the buyers in the queue. To this end, we only
focus on (measurable) subsets of V of the form:
Vk(v) := {v : vk+1 ≤v},
which comprises a set of queue states in which the k + 1-st highest value is less than v,
or equivalently, at most k buyers have values above v. Figure 7 depicts V0(v) and V1(v)
37

(only in the first two coordinates).
1
v
1
V0(v)
v1 = v2
≤λ[1 −F (v)]p0(v)
≥µy1(v)p1(v)
v1
v2
1
1
v
V1(v)
v1 = v2
v1
v2
Figure 7: V0(v) and V1(v)
A balance condition for Vk(v) boils down to:
λ[1 −F(v)]pk(v) ≥µyk+1(v)pk+1(v).
(Bk)
The LHS is an upper bound on the outflow from set Vk(v), depicted for k = 0 by the
out-arrow. Ignoring the higher order terms, the outflow occurs when there are exactly
k buyers with values strictly above v—an event that occurs with probability pk(v)—and
a buyer with value above v arrives—which occurs at rate λ[1 −F(v)]. In that case, a
transition out of Vk(v) occurs if that buyer is admitted into the queue. Since he may not,
the LHS gives an upper bound for the outflow. Analogously, the RHS gives inflow into
the set Vk(v).
These conditions here apply only to lower-dimensional subsets of measurable sets
that characterize the stationary distribution p ∈∆(Θ). To be more precise, Vk(v) is
indexed by each (k, v), so we have cardinality N × [0, 1] of conditions. But to account
for all measurable sets, we need a condition for a set of the form [0, v], for each v ∈
V, so the cardinality of the conditions becomes in the order of N[0,1], far bigger than
N × [0, 1]. Clearly, the selected conditions are necessary. Although they are not sufficient
for characterizing the stationary distribution, they turn out to be sufficient for identifying
38

the optimal solution to [P′
TU] (that will be stated below); as only they will be seen to bind
at the optimal solution. The reduction is ultimately what makes the problem tractable.
We are now ready to formulate our relaxed program. Consider the problem:
max
p,y,z,X,T λ
Z 1
0
[αU M(v) + (1 −α)T(v)]f(v)dv −c
∞
X
k=1
kpk(0) −d
∞
X
ℓ=1
ℓp−ℓ
subject to (IR), (IC), (RF), (Bℓ), and (Bk), ∀ℓ, k.
Again, one can interpret the objective as the long-run time average of the weighted sum of
consumer and designer/producer surplus (with α being the weight for the former) minus
the buyer waiting costs the designer reimburses and inventory costs, or its counterpart for
steady-state flow surplus. Note that the term P∞
k=1 kpk(0) accounts for the steady-state
average queue length: pk(0) is the stationary probability that there are k buyers in the
queue (given our convention to encode absenece of a buyer by a presence of buyer with
value 0).
Clearly, all constraints are necessary for a mechanism to be in M∗∗. Using the stan-
dard envelope condition, the problem is further relaxed to:
max
p,y,z,X λ
Z 1
0
J(v)X(v)f(v)dv −c
∞
X
k=1
kpk(0) −d
∞
X
ℓ=1
ℓp−ℓ
[P′
TU]
subject to (RF), (Bℓ), and (Bk), ∀ℓ, k,
where J(v) := v −(1 −2α)1−F(v)
f(v) , which we assume is increasing in v.
Upon suitable change of variables, [P′
TU] can be seen as a linear program. Note that we
have transformed a dynamic mechanism design problem into a linear optimization prob-
lem. What made this transformation possible is the combination of traditional mechanism
design tools, such as (IR) and (IC), with the reduced-form characterization (RF) and
the balance conditions necessitated by stationarity. While the approach here is similar
in spirit to its NTU counterpart, [P′
NTU], the presence of private information, and more
importantly, the richness of the queue state space Θ sets it apart. As mentioned ear-
39

lier, reducing the balance conditions for stationarity is a key step toward a tractable LP
formulation. The optimal solution to [P′
TU] is characterized next.
Theorem 9. The optimal solution to the relaxed program [P′
TU] is characterized as
follows: There are ˆvK > · · · > ˆv1 > ˆv−1 > · · · > ˆv−L > J−1(0), for some K, L ∈N, such
that (i) items are stored only up to L units; (ii) buyers are queued up to k ≤K buyers if
and only if all k of them have values above ˆvk; (iii) if a buyer arrives with ℓ≥1 items in
storage, he is allocated the good if and only if his value is above ˆv−ℓ; and (iv) if an item
arrives with k ≥1 buyers waiting in the queue, then it is assigned to the buyer with the
highest value.
Proof. The proof involves identifying the set of primal and dual variables that satisfy the
set of all complementary slackness. The interested reader is referred to Che and Choi
(2025) for details.
Similar to the static mechanism design, the optimal dynamic mechanism allocates the
items optimally among (endogenously selected) participants. Unlike the static mecha-
nism, the key aspect of the optimal dynamic mechanisms concerns the design of queues.
This aspect is crucial not only for balancing the intertemporal mismatch between de-
mand and supply, but also for allocating competitive pressures across time to effectively
discipline privately informed buyers. Since both buyers and items are costly to store in
queues, the mechanism requires buyers to meet progressively higher cutoffs as the queue
length increases and the number of inventoried goods falls.
Note that the qualitative features of the optimal allocation are similar for revenue
maximization (α = 0) and welfare maximization (α = 1/2). The only difference between
the two is that queue-dependent cutoffs ˆvk differ between the two cases. In the revenue
maximization case, they are chosen to maximize the virtual value J(v; α = 0) = v −
1−F(v)
f(v) , just as in the Myerson setting, whereas in the welfare maximization case, they
are chosen to maximize realized value J(v; α = 1/2) = v. Since the welfare-maximizing
designer lacks the monopoly exclusion motive, one would expect the latter to be lower in
40

general. This is indeed true for a low queue size. For example, given d = ∞, the welfare
maximizing cutoff for one buyer queue is ˆv1 = c, whereas the corresponding revenue-
maximizing cutoff is ˆv1 = J−1(c), the same as the standard Myerson reserve-price with
cost c. More surprisingly, however, for a large queue size, the order is reversed: the
revenue-maximizing cutoffs are lower than the welfare-maximizing cutoffs. This single
crossing feature can be explained as follows.
Again, lacking the monopoly exclusion
motive, the welfare-maximizing designer is more willing to admit buyers into the queue
when the queue is relatively short, thereby reducing the risk of buyer stockout (i.e., the
situation where no buyers are available when a good arrives). At the same time, the
revenue-maximizing designer is more willing to admit buyers into a long queue, since she
is more keen on selling to high-value buyers who command lower information rent, and
because the higher exclusion for shorter queues already means a stronger need to insure
against the buyer stockout risk. Consequently, compared to welfare-optimal thresholds,
revenue-maximizing thresholds are higher for short queues and lower for long queues.
Theorem 9 characterizes the optimal solution to [P′
TU]. What remains is to show that
the optimal solution to this relaxed problem can be made incentive compatible by a
mechanism M satisfying the constraints of our original program [PTU]. Che and Choi
(2025) demonstrates that a mechanism comprising a series of auctions can implement the
optimal mechanism in dominant strategies.
To illustrate how the optimal mechanism works, suppose a buyer arrives following a
null state initially. Then, a designer imposes a reserve price of ˆv1, a minimum price he
will be charged for a good he may receive, making it dominant for the buyer to join the
queue if and only if his value is above ˆv1. Suppose another buyer arrives before an item
arrives. Then, the reserve price, or an ascending auction clock, rises from ˆv1 continuously
until one of them drops out when the price reaches his value, or until it reaches ˆv2,
whichever happens first. In the first case, the price is stopped at the drop-out price, with
the surviving buyer remaining in the queue. In the latter case, both buyers survive and
stay in the queue. More generally, if a buyer arrives at a queue with length k, then a
41

clock price will similarly rise from the existing stopped price until a buyer drops out or
a price of ˆvk+1 is reached, whichever occurs first.
Suppose a good arrives next with buyers waiting. Then, a sealed-bid auction is held
in which buyers are required to make a bid no less than the highest reserve prices they
have survived. The highest bidder wins the good (with ties broken at random) and is
required to pay the cutoff price—defined as the lowest bid he could have made and
would have eventually won in light of the future sample path, assuming that the same
bid is used to determine future assignments. A cutoff price depends on the sample path
realized (possibly well) after the auction, reflecting the realized competition the designer
faces afterwards. In this dynamic setting, the price is not a single number known at the
moment of matching, but a function of the realized stochastic process.39 In this case, the
winner is “billed” after receiving the good.
Intuitively, the cutoff prices track the future level of competition to discipline the
current buyers. If the supply condition improves (with the arrival of more items) following
the assignment, a low price will be charged to the winner. By contrast, if excess demand
arises, the winner will be charged a high price.
Next, the designer caps the size of the inventory at some finite L units; an additional
item received after reaching L is discarded. Suppose there is an inventory of ℓ≥0 items,
and a buyer arrives. Then, the buyer is charged a fixed price of ˆv−ℓ; as noted before, the
price is lower the larger the inventory.
The auction/pricing mechanism described here implements the optimal outcome in
dominant strategies. While we allowed for all PRRMs, the optimal DSIC mechanism is
pseudo-Markovian: its allocation and queue/inventory decisions are Markovian, depend-
ing only on the queue state θ ∈Θ, whereas the cutoff price depends possibly on the entire
within-cycle history. An implication is that the allocation depends only on the reported
39To fix the idea, consider a winner, A, who arrives when another buyer, B, is already in the queue.
In one scenario, a second item arrives later to satisfy B; here, A’s payment is low because the eventual
supply was sufficient for both. In a second scenario, no second item arrives, but two new high-value
buyers arrive instead. In this case, A must pay a higher price because, in light of that realized sample
path, A faced much tougher competition to secure one of the scarce items.
42

values of the buyers in the queue, independently of their arrival orders; in other words,
the allocation priority does not follow the standard queue discipline.
In summary, the optimal dynamic mechanism retains the core element of Myerson
(1981) — namely, allocating goods to buyers who are already present via an auction with
a reserve price. Furthermore, it involves an additional feature whereby the designer stores
buyers or goods in a queue to balance the potential intertemporal mismatches between
demand and supply, and, no less importantly, to balance competitive pressures across
time.
3.3
Large Market Properties
Suppose the market becomes dense as λ, µ →∞with the balance parameter ρ = λ/µ
held constant, or as c or d vanishes. As with the NTU model, the limit of such markets
corresponds to a static model with a unit mass of items on the supply side and a mass
ρ = λ/µ of buyers on the demand side (see Che and Choi (2025)), or a dynamic model
in which a unit mass of items and a mass ρ = λ/µ of buyers arrive at each instant.
The optimal mechanism in this limit continuum model is very simple. Focusing on
revenue maximization (i.e., α = 0), the optimal mechanism is simply the uniform-price
multiunit auction with optimal reserve price: i.e., selling the mass of items at price equal
to either the marginal buyer’s value ˜v := inf{v : ρ[1 −F(v)] ≤1}, or the standard
monopoly price ˆv0 := J−1(0), whichever is higher. (The former is the continuum market
analog of the highest losing bid in the uniform-price auction.) Note also that, as α →1/2,
ˆv0 →0.
Figure 8 features the situation in which the former price is higher.
43

1
1
˜v
˜v0 + δ
ˆv
λ
µ[1 −F(v)]
v
Figure 8: Large market limit of the optimal mechanism
Che and Choi (2025) establishes the following results.
Theorem 10. The normalized objective converges to the continuum model optimum if
(a) λ, µ →∞with ρ = λ/µ held constant, or (b) c →0, or (c) d →0.
Proof Sketch: Multiplying the four parameters λ, µ, c, and d by a constant k > 0 is
equivalent to simply rescaling time. Hence, (b) is the same as λ, µ, d →∞with λ/µ and
λ/d held constant, and (c) is the same as λ, µ, c →∞with λ/µ and λ/c held constant.
Thus (a) is implied by either (b) or (c).
To prove (b), consider a simple feasible mechanism that assigns an incoming good to
a waiting buyer if the buyer queue is non-empty and discards the good otherwise, and
queues a buyer if and only if his value is above ˜v0 + δ for an arbitrarily small δ > 0 (see
Figure 8). This ensures that the good arrival rate µ is greater than an ”effective” buyer
arrival rate λ[1 −F(˜v0 + δ)]. Then, every buyer with a value above ˜v0 + δ is eventually
assigned a good after a delay; however, one can show that the delay vanishes as c →0.
Therefore, the per-unit revenue converges to the level attained in the optimal multiunit
auction mechanism in the continuum model. This indicates that the optimal mechanism
must also converge to the latter, which can be shown to provide the upper bound in the
limit of a large market.
44

Similarly, to prove (c), we consider a mechanism that sells a stored item to an incoming
buyer at a price ˜v0 + δ, turns away buyers when inventory is empty, and stores at most
¯L :=
p
µ/d items. As d →0, one can show both the average storage cost borne by the
designer and the probability of stockout (conditional on buyer arrival) vanish.
One can see a stark yet obvious difference relative to the NTU case. Recall from
Theorem 8 that a social optimum in the NTU setting involves a complete pooling or
random assignment; here, the social optimum (α = 1/2) yields allocative efficiency.
4
Broader Overview and Future Directions
This survey has focused on a framework of steady-state mechanism design. This approach
provides a unified method for analyzing the core trade-offs in settings with stochastic and
asynchronous arrivals. However, the broader agenda of dynamic market design is rich and
varied, and many important contributions approach the problem from different angles or
with various assumptions. To place the surveyed work in a broader context, we briefly
discuss six related streams of literature before turning to promising directions for future
research.
Other Perspectives on Queueing and Priority
The question of optimal queue
design has a long history. The framework presented here, which enables the joint opti-
mization of entry, priority, and information, often yields different conclusions than studies
that restrict one or more of these design levers. For example, some work has found FCFS
to be optimal in models where the stochasticity of the queue length is muted, either by
assuming a continuum of agents or specific arrival-departure processes that render the
queue size deterministic (e.g., Bloch and Cantala (2017); Margaria (2020)). Other re-
search focuses on different sources of uncertainty; Cripps and Thomas (2019), for instance,
analyzes a setting where the service rate is unknown, leading to a problem of strategic
experimentation by agents, a different challenge from managing incentives based on the
known state of the system.
45

Screening with State-Independent Mechanisms
A significant literature has pio-
neered the application of mechanism design to queueing systems, particularly for screen-
ing agents with heterogeneous preferences. Key contributions include Mendelson and
Whang (1990), Afeche (2013), Afeche and Pavlin (2016), and Kittsteiner and Moldovanu
(2005). These papers study how a service provider can design a menu of options—typically
price-priority or price-delay pairs—to induce self-selection. The fundamental difference
between this work and the TU framework surveyed here lies in whether the mechanism
itself is dynamic. The mechanisms in the aforementioned literature are typically state-
independent; the menu of contracts offered to an arriving agent is fixed and does not
change with the number of customers already waiting or the system’s current state.
Dynamic Matching.
The problems discussed in this survey is related to the broad
domain of dynamic matching, a field that has seen an explosion of recent work (e.g.,
Akbarpour, Li, and Gharan (2020), Akbarpour, Combe, Hiller, Shimer, and Tercieux
(2020), Ashlagi, Nikzad, and Strack (2023), to name just a few). This literature has
often focused on questions of aggregate market performance, such as the value of “market
thickness” or the optimal timing of batch-matching versus continuous matching. The
research highlighted in this survey complements this work by taking a more granular,
micro-level mechanism design approach. The focus is less on the aggregate timing of
matches and more on the precise design of policies—entry control, priority assignment,
and information disclosure—needed to manage the participation and waiting incentives
of individual, strategic agents.
Also noteworthy is a distinct literature that takes a
cooperative games approach to define stability in a dynamic setting; see Damiano and
Lam (2005) and Doval (2022) for example. These works grapple with the conceptual
challenges of extending static stability to a dynamic context, such as defining credible
multi-period blocking plans and formalizing agents’ expectations about the future.
Platform Management of Frictional Matching.
A significant literature studies
platforms that manage the inefficiencies inherent in decentralized, frictional matching
46

markets.
In these models, agents actively search and incur costs to screen potential
partners, which can lead to congestion and wasted effort when they contact others who
are ultimately unavailable or uninterested (Arnosti, Johari, and Kanoria (2021); Fradkin
(2017); Horton (2019)). Here, the platform’s role is not to operate a centralized queue, but
to indirectly manage these search frictions by designing the search environment itself. A
key lever for the platform is to guide the search process by restricting the set of potential
partners an agent might meet. Interventions include designing the search protocol by
restricting who can initiate contact (Kanoria and Saban (2021)), or by directly setting
the meeting rates between different types of agents (Immorlica, Lucier, Manshadi, and
Wei (2021)). This approach contrasts sharply with the framework of this survey, which
assumes a powerful designer with direct, centralized control over the allocation process
via a queue, where frictions primarily manifest as waiting costs within a managed system.
Dynamic Pricing and Revenue Management
The surveyed works are related
to classic literature on dynamic pricing and revenue management (e.g., Gallego and
Van Ryzin (1994); Board and Skrzypacz (2016); Gershkov, Moldovanu, and Strack (2018);
Pai and Vohra (2013), and Dilme and Li (2019)). This literature typically addresses the
problem of selling a fixed, perishable inventory (like airline seats) to stochastically arriv-
ing buyers over a finite time horizon. In that setting, the dynamics are driven by the
non-stationarity of a depleting stock and an approaching deadline. The framework in
this survey analyzes a different economic environment: an infinite-horizon system where
stochasticity is present on both sides of the market (i.e., supply and demand are both
random flows).
The central problem is thus the management of long-run, stationary
processes, rather than the optimal pricing path for a finite deadline.
Dynamic Mechanism with Evolving Private Information.
A major branch of the
literature on dynamic mechanism design, including works by Courty and Li (2000), Es¨o
and Szentes (2007), Bergemann and V¨alim¨aki (2010), Athey and Segal (2013), Pavan,
Segal, and Toikka (2014), Bergemann and Strack (2015), and Bergemann and Strack
47

(2022), addresses a different source of dynamics. In this paradigm, the set of agents is
fixed, but their private information—their “type”—evolves stochastically over time, often
as a function of their past allocations (e.g., through learning-by-doing or consuming an
experience good). The central design problem is thus not the management of market
flows, but the characterization of optimal long-term contracts that provide intertemporal
incentives for truth-telling as this information evolves. This focus leads to a different
set of analytical tools centered on dynamic versions of the first-order approach, which
contrasts with the steady-state and queueing theory methods central to this survey. See
Chapter 11 of B¨orgers (2015) and Bergemann and V¨alim¨aki (2019) for excellent surveys
on the subject matter.
Future Directions
Several promising avenues for research emerge from relaxing the
core assumptions of the models discussed.
First, in the NTU setting, a key challenge is to extend the analysis to accommodate
heterogeneous agent types in a non-fluid model. The current state of the art for this
problem, such as in Ashlagi, Monachou, and Nikzad (2025), largely invokes a fluid or
continuum-agent model.
While tractable, this approach effectively assumes away the
very stochastic frictions and integer-level queue dynamics that make waiting a complex
and interesting problem in the first place. Progress in a non-fluid model is challenging
because feasible allocation of waiting times among stochastically arriving agents and
items is difficult to characterize.
Second, in the TU setting, while the model in Che and Choi (2025) allows for hetero-
geneous values, there is ample scope to incorporate richer forms of heterogeneity, such as
private information about agents’ waiting costs, outside options, or their specific service
requirements. For example, in many gig platforms, tasks differ not only in the value
they create but also in the time they take to complete. Designing mechanisms that can
effectively screen agents along these multiple dimensions will require new methodolog-
ical tools to analyze the interplay between queueing, multi-dimensional screening, and
48

dynamic state-contingent policies.
Both areas represent exciting frontiers for market design.
Understanding how to
structure dynamic marketplaces—whether through waiting, prices, or a combination of
both—remains a central task for economic theory and a practical imperative for the
modern digital economy.
References
Afeche, P. (2013): “Incentive-compatible revenue management in queueing systems:
Optimal strategic delay,” Manufacturing & Service Operations Management, 15(3),
423–443.
Afeche, P., and J. M. Pavlin (2016): “Optimal price/lead-time menus for queues with
customer choice: Segmentation, pooling, and strategic delay,” Management Science,
62(8), 2412–2436.
Akbarpour, M., J. Combe, V. Hiller, R. Shimer, and O. Tercieux (2020): “Un-
paired Kidney Exchange: Overcoming Double Coincidence of Wants without Money,”
NBER Working paper number 27765.
Akbarpour, M., S. Li, and S. O. Gharan (2020): “Thickness and information in
dynamic matching markets,” Journal of Political Economy, 128(3), 783–815.
Anunrojwong, J., K. Iyer, and V. Manshadi (2020): “Information Design for
Congested Social Services: Optimal Need-Based Persuasion,” EC ’20: Proceedings of
the 21st ACM Conference on Economics and Computation, pp. 349–350.
Arnosti, N., R. Johari, and Y. Kanoria (2021): “Managing congestion in matching
markets,” Manufacturing & Service Operations Management, 23(3), 620–636.
Arnosti, N., and P. Shi (2020): “Design of lotteries and wait-lists for affordable
housing allocation,” Management Science, 66(6), 2291–2307.
49

Ashlagi, I., F. Monachou, and A. Nikzad (2025): “Optimal allocation via waitlists:
Simplicity through information design,” Review of Economic Studies, 92(1), 40–68.
Ashlagi, I., A. Nikzad, and P. Strack (2023): “Matching in dynamic imbalanced
markets,” The Review of Economic Studies, 90(3), 1084–1124.
Asmussen, S. (2003): Applied Probability and Queues. Springer-Verlag.
Athey, S., and I. Segal (2013): “An Efficient Dynamic Mechanism,” Econometrica,
81, 2463–2485.
Baccara, M., S. Lee, and L. Yariv (2020): “Optimal dynamic matching,” Theoret-
ical Economics, 15, 1221–1278.
Becker, G. S. (1973): “A theory of marriage: Part I,” Journal of Political economy,
81(4), 813–846.
Bergemann, D., and P. Strack (2015): “Dynamic revenue maximization: A contin-
uous time approach,” Journal of Economic Theory, 159, 819–853.
(2022): “Progressive participation,” Theoretical Economics, 17(3), 1007–1039.
Bergemann, D., and J. V¨alim¨aki (2010): “The dynamic pivot mechanism,” Econo-
metrica, 78(2), 771–789.
(2019): “Dynamic mechanism design: An introduction,” Journal of Economic
Literature, 57(2), 235–274.
Bloch, F., and D. Cantala (2017): “Dynamic assignment of objects to queuing
agents,” American Economic Journal: Microeconomics, 9, 88–122.
Board, S., and A. Skrzypacz (2016): “Revenue management with forward-looking
buyers,” Journal of Political Economy, 124(4), 1046–1087.
Border, K. C. (1991): “Implementation of reduced form auctions: A geometric ap-
proach,” Econometrica: Journal of the Econometric Society, pp. 1175–1187.
50

B¨orgers, T. (2015): An introduction to the theory of mechanism design. Oxford uni-
versity press.
Castro, F., H. Ma, H. Nazerzadeh, and C. Yan (2021): “Randomized FIFO
mechanisms,” arXiv preprint arXiv:2111.10706.
Che, Y.-K., and A. B. Choi (2025): “Optimal Auction Design for Dynamic Stochastic
Environments: Myerson Meets Naor,” arXiv preprint arXiv:2505.22862.
Che, Y.-K., D. Condorelli, and J. Kim (2018): “Weak cartels and collusion-proof
auctions,” Journal of Economic Theory, 178, 398–435.
Che, Y.-K., J. Kim, and K. Mierendorff (2013): “Generalized Reduced-Form Auc-
tions: A Network-Flow Approach,” Econometrica, 81(6), 2487–2520.
Che, Y.-K., and O. Tercieux (forth.): “Optimal Queue Design,” Journal of Political
Economy.
Condorelli, D. (2012): “What money can’t buy: Efficient mechanism design with
costly signals,” Games and Economic Behavior, 75(2), 613–624.
Courty, P., and H. Li (2000): “Sequential screening,” The Review of Economic Stud-
ies, 67(4), 697–717.
Cripps, M. W., and C. D. Thomas (2019): “Strategic experimentation in queues,”
Theoretical Economics, 14, 647–708.
Damiano, E., and R. Lam (2005): “Stability in dynamic matching markets,” Games
and Economic Behavior, 52(1), 34–53.
Dilme, F., and F. Li (2019): “Revenue management without commitment: Dynamic
pricing and periodic flash sales,” The Review of Economic Studies, 86(5), 1999–2034.
Doval, L. (2022): “Dynamically stable matching,” Theoretical Economics, 17(2), 687–
724.
51

Es¨o, P., and B. Szentes (2007): “Optimal information disclosure in auctions and the
handicap auction,” The Review of Economic Studies, 74(3), 705–731.
Fradkin, A. (2017): “Search, matching, and the role of digital marketplace design in
enabling trade: Evidence from airbnb,” .
Gale, D., and L. S. Shapley (1962): “College admissions and the stability of mar-
riage,” The American Mathematical Monthly, 69(1), 9–15.
Gallego, G., and G. Van Ryzin (1994): “Optimal dynamic pricing of inventories
with stochastic demand over finite horizons,” Management science, 40(8), 999–1020.
Gershkov, A., B. Moldovanu, and P. Strack (2018): “Revenue-maximizing mech-
anisms with strategic customers and unknown, markovian demand,” Management Sci-
ence, 64(5), 2031–2046.
Hartline, J. D., and T. Roughgarden (2008): “Optimal mechanism design and
money burning,” in Proceedings of the fortieth annual ACM symposium on Theory of
computing, pp. 75–84.
Hassin, R. (1985): “On the optimality of first come last served queues,” Econometrica,
53, 201–202.
Hassin, R., and M. Haviv (2003): To Queue or Not to Queue: Equilibrium Behavior
in Queueing Systems. Kluwer Academic Publishers.
Horton, J. J. (2019): “Buyer uncertainty about seller capacity: Causes, consequences,
and a partial solution,” Management Science, 65(8), 3518–3540.
Immorlica, N., B. Lucier, V. Manshadi, and A. Wei (2021): “Designing ap-
proximately optimal search on matching platforms,” in Proceedings of the 22nd ACM
Conference on Economics and Computation, pp. 632–633.
Kanoria, Y., and D. Saban (2021): “Facilitating the search for partners on matching
platforms,” Management Science, 67(10), 5990–6029.
52

Kittsteiner, T., and B. Moldovanu (2005): “Priority auctions and queue disciplines
that depend on processing time,” Management Science, 51(2), 236–248.
Kleiner, A., B. Moldovanu, and P. Strack (2021): “Extreme points and ma-
jorization: Economic applications,” Econometrica, 89(4), 1557–1593.
Larson, R. C. (1987): “Perspectives on queues: social justice and the psychology of
queueing,” Operations Research, 35, 895–905.
Leshno, J. (2019): “Dynamic matching in overloaded waiting lists,” Discussion paper,
SSRN Working Paper 2967011.
(2022): “Dynamic matching in overloaded waiting lists,” American Economic
Review, 112(12), 3876–3910.
Lingenbrink, D., and K. Iyer (2019): “Optimal signaling mechanisms in unobservable
queues,” Operations Research, 67, 1397–1416.
Madsen,
E.,
and
E.
Shmaya (2025):
“Collective Upkeep,”
arXiv preprint
arXiv:2407.05196.
Margaria, C. (2020): “Queueing to learn,” Discussion paper, Boston University.
McAfee, R. P., and J. McMillan (1992): “Bidding rings,” The American Economic
Review, pp. 579–599.
Mekonnen, T. (2019): “Random vs. Directed Search for Scarce Resources,” Working
Paper.
Mendelson, H., and S. Whang (1990): “Optimal incentive-compatible priority pric-
ing for the M/M/1 queue,” Operations research, 38(5), 870–883.
Milgrom, P. R., and R. J. Weber (1982): “A theory of auctions and competitive
bidding,” Econometrica: Journal of the Econometric Society, pp. 1089–1122.
53

Myerson, R. B. (1981): “Optimal auction design,” Mathematics of Operations Re-
search, 6(1), 58–73.
Naor, P. (1969): “The regulation of queue size by levying tolls,” Econometrica, 37,
15–24.
Pai, M. M., and R. Vohra (2013): “Optimal dynamic auctions and simple index
rules,” Mathematics of Operations Research, 38(4), 682–697.
Pavan, A., I. Segal, and J. Toikka (2014): “Dynamic Mechanism Design: A Myer-
sonian Approach,” Econometrica, 82(2), 601–653.
Schummer, J. (2021): “Influencing waiting lists,” Journal of Economic Theory, 195,
105263.
Shanthikumar, J. G., and U. Sumita (1987): “Convex ordering of sojourn times
in single-server queues: extremal properties of FIFO and LIFO service disciplines,”
Journal of Applied Probability, 24, 737–748.
Shapley, L. S., and M. Shubik (1971): “The assignment game I: The core,” Interna-
tional Journal of game theory, 1(1), 111–130.
Su, X., and S. Zenios (2004): “Patient choice in kidney allocation: The role of the
queueing discipline,” Manufacturing and Services Operations Management, 6, 280–301.
Thorisson, H. (1992): “Construction of a stationary regenerative process,” Stochastic
Processes and their Applications, 42(2), 237–253.
Wolff, R. W. (1982): “Poisson Arrivals See Time Averages,” Operations Research, 30,
223–231.
54

Online Appendix
A
Regenerative Processes
Given a stochastic process, which we denote as θ = {θ(t) : t ≥0}, we can imagine that
there exists a specific, random time τ1 at which the process probabilistically ”starts over”.
If the evolution of the process from this time onward, {θ(τ1 + t) : t ≥0}, has the same
distribution as the original process and is independent of its past history, then we say
that the process θ has regenerated at time τ1. The portion of the process’s evolution
within the interval [0, τ1), along with the regeneration time τ1 itself, is called the first
cycle, denoted C1 = {{θ(t) : 0 ≤t < τ1}, τ1}. The duration of this cycle is the first cycle
length, T1 = τ1.
If such a regeneration time τ1 exists, the nature of the process implies that this
restarting behavior will continue. There must be a second regeneration time τ2 > τ1,
which marks the end of a second cycle, C2, that is identically distributed to the first.
Continuing this logic, we find a sequence of regeneration times {τk : k ≥1} (with
τ0 = 0) that constitutes a renewal process. The cycle lengths Tk = τk −τk−1 for k ≥1
are independent and identically distributed (i.i.d.), and consequently, the cycles Ck =
{{θ(τk−1 + t) : 0 ≤t < Tk}, Tk} are also i.i.d. objects.
A regenerative process is said to be positive recurrent if the underlying renewal
process of regeneration times is also positive recurrent; this is the case when the expected
cycle length is finite and positive, or 0 < E[T1] < ∞. If the expected cycle length is
infinite (E[T1] = ∞), the process is called null recurrent.
Because the regeneration times {τk} form a renewal process and the cycles {Ck} are
i.i.d., one can compute long-run time averages, which will be seen to equal the expected
value over a cycle divided by its expected length. If θ is a regenerative process, it follows
that for any measurable function f(x), the transformed process f(θ(t)) is also a regener-
ative process that shares the exact same regeneration times. This property allows us to
55

establish a general theorem for the time average of such functions.
The main result is stated precisely in the following theorem.
Theorem A.11. If θ is a positive recurrent regenerative process, and f = f(x) is a
function such that E[
R T1
0
|f(θ(s))|ds] < ∞, then the following hold:
• The long-run time average of the process converges with probability 1:
lim
t→∞
1
t
Z t
0
f(θ(s))ds = E[Y ]
E[T]
• The limit of the time-averaged expectation also converges to the same value:
lim
t→∞
1
t
Z t
0
E[f(θ(s))]ds = E[Y ]
E[T]
Here, T = T1 is the first cycle length and Y = Y1 =
R T1
0
f(θ(s))ds is the value of f(θ)
accumulated during the first cycle.
Proof. Let N(t) = max{j : τj ≤t} denote the number of regenerations by time t, and let
Yj =
R τj
τj−1 f(θ(s))ds be the reward from the jth cycle. The proof for the first part of the
theorem proceeds in two steps.
Assume first the function f is non-negative (f ≥0). Since time t must fall between
the N(t)-th and (N(t) + 1)-th regeneration, we can bound the integral of f(θ(s)). The
total reward is at least the sum of rewards from all completed cycles and at most the sum
of rewards from all completed cycles plus the reward from the entire next cycle. This
gives the sandwich inequality:
1
t
N(t)
X
j=1
Yj ≤1
t
Z t
0
f(θ(s))ds ≤1
t
N(t)+1
X
j=1
Yj
We examine the lower bound first by rewriting it as a product:
1
t
N(t)
X
j=1
Yj = N(t)
t
·
1
N(t)
N(t)
X
j=1
Yj
56

As t →∞, the term N(t)
t
converges to
1
E[T] by the Elementary Renewal Theorem. The
second term, being the average of i.i.d. random variables Yj with a finite mean (as per the
theorem’s hypothesis), converges to E[Y ] by the Strong Law of Large Numbers. Thus,
the lower bound converges to E[Y ]
E[T].
Similarly, the upper bound can be rewritten as
N(t)+1
t
·
1
N(t)+1
PN(t)+1
j=1
Yj.
Since
N(t)+1
t
→
1
E[T], the upper bound also converges to the same limit E[Y ]
E[T]. By the Squeeze
Theorem, the integral term must also converge to E[Y ]
E[T], which proves the result for the
non-negative case. As a direct consequence, the difference between the upper and lower
bounds, which is precisely
YN(t)+1
t
, must converge to 0 with probability 1.
Suppose next that function f need not be non-negative. We can apply the result from
the first case to the non-negative function |f|. Let Y ∗
j =
R τj
τj−1 |f(θ(s))|ds. From the prior
step, we know that
Y ∗
N(t)+1
t
→0 with probability 1.
Now, we decompose the integral of f into the sum over complete cycles and the
remainder in the last, incomplete cycle:
1
t
Z t
0
f(θ(s))ds = 1
t
N(t)
X
j=1
Yj + 1
t
Z t
τN(t)
f(θ(s))ds
The first term on the right-hand side converges to E[Y ]
E[T], as shown before. The second term
is the error term, which we must show converges to 0. We can bound its magnitude:

1
t
Z t
τN(t)
f(θ(s))ds
 ≤1
t
Z t
τN(t)
|f(θ(s))|ds ≤1
t
Z τN(t)+1
τN(t)
|f(θ(s))|ds =
Y ∗
N(t)+1
t
Since we established that
Y ∗
N(t)+1
t
→0, the error term converges to 0. This completes the
proof for the first part of the theorem.
A straightforward application of Theorem A.11 characterizes the stationary distribu-
tion.
Corollary A.1. Suppose P is the stationary distribution of θ = {θ(t) : t ≥0}, a positive
57

recurrent regenerative process. Then, for each measurable set Θ′ ⊂Θ,
Pr{θ ∈Θ′} = lim
t→∞
1
t
Z t
0
1{θ(s)∈Θ′}ds = E[Y ]
E[T],
with probability 1
and also,
Pr{θ ∈Θ′} = lim
t→∞
1
t
Z t
0
Pr{θ ∈Θ′}ds = E[Y ]
E[T],
where T = T1 and Y =
R T1
0
1{θ(s)∈Θ′}ds. Hence, P is also a unique limiting distribution
of θ(t).
Proof. We first note that a stationary distribution coincides with a long run time average.
We apply Theorem A.11 by setting f(x) = 1{x∈Θ′}, which satisfies the conditions of the
theorem.
We do not present the limit theorem for a regenerative process, as it requires additional
background knowledge (e.g., the key renewal theorem). An interested reader is referred
to Asmussen (2003).
B
Proof of Theorem 3
Proof. Suppose the first incumbent decides to leave the queue if and only if the queue
reaches a length K.(That the optimal decision takes this form is obvious.)
We can
characterize the value wk to the first incumbent of the queue length (including herself)
being k = 1, ..., K via dynamic programming. First, observe that
w1 = (µdt)v + (λdt)w2 + (1 −µdt −λdt)(w1 −cdt) + o(dt),
since during the next short time interval dt, either he receives his service and collects v
with probability µdt or a new entrant arrives with probability λdt and his value shifts
to w2, or neither happens with the remaining probability, in which case he stays at the
58

same state except that he incurs the waiting costs cdt. Similarly, we can write:
wk = (µdt)wk−1 + (λdt)wk+1 + (1 −µdt −λdt)(wk −cdt) + o(dt), ∀k = 2, ..., K −1;
wK = (µdt)wK−1 + (1 −µdt −λdt)(wK −cdt) + o(dt),
where we use the fact that when a new entry occurs at k = K, the first incumbent
immediately exits the queue.
Dividing all equations by dt and letting dt →0, we obtain a system of equations:
(λ + µ)w1 = µv + λw2 −c;
(λ + µ)wk−1 = µwk−2 + λwk −c, ∀k = 2, ..., K −1;
(λ + µ)wK = µwK−1 −c.
This system has a unique solution. In particular, we focus on its last coordinate:
wK = pK
0

v −
K−1
X
j=0
(K −j)c
µ
ρj
!
,
for the first incumbent stays in the queue up to length K if and only if wK ≥0. Namely,
the maximal queue length KLCFS under LCFS satisfies wKLCFS ≥0 ≥wKLCFS+1.
Define the social welfare under the cutoff structure with the maximal queue length as
W(K) =
K
X
k=1
pK
k (µv −ck).
A straightforward calculation confirms that
∆W(K) := W(K) −W(K −1)
∝v −
K−1
X
j=0
(K −j + 1)c
µ
ρj
∝wK.
59

Hence, wK ≥0 if and only if ∆W(K) ≥0. Together with the fact that W(K) is quasi-
concave in K, the conclusion follows.
C
Proof of Theorem 4
Proof. We can write the expected waiting time when one finds herself in the queue of k,
or joining the queue of k:C.40
τk = (µdt)
k −1
k
τk−1

+ (λdt)τk+1 + (1 −µdt −λdt)[τk + dt] + o(dt), ∀k < K;
τK = (µdt)
K −1
K
τK−1

+ (1 −µdt)[τK + dt] + o(dt),
where τ0 ≡0. Dividing both sides by dt and letting dt →0, we get
(µ + λ)τk = µ
k −1
k
τk−1

+ λτk+1 + 1, ∀k < K;
(C.2)
µτK = µ
K −1
K
τK−1

+ 1.
(C.3)
The online appendix of Che and Tercieux (forth.) (which considers more general primitive
processes) shows that the system admits a unique solution which satisfies
1
µ ≤τ1 ≤.... ≤τK ≤K
µ .
In fact, we can show that K < K
µ ; or else (C.3) implies that τK = τK−1 = K/µ, which,
when applied to the penultimate equation, yields τK−2 = (K−1)2
Kµ
> K
µ = τK, a contradic-
tion to the monotonicity. (Analogously, one can show that τ1 > 1/µ.) Since KSIR0 is the
largest K such that v −cτK ≥0 and K < K
µ , the K-th entrant’s wait time under FCFS,
we conclude that KSIRO ≥KFCFS. To show KSIRO ≤K∗, we can assume K∗< ∞without
loss. In the case, (IRB) is binding at K∗. Clearly, (IRB) is satisfied under SIRO. To see
C.40They are the recursive equations applying the logic of dynamic programming, where we use the fact
that unless state changes (when a new buyer arrives with k < K or a good arrives), the state remains
the same with elapse of waiting time by dt, for a brief period dt.
60

this, write:
KSIRO
X
k=1
pKSIRO
k
(µv −ck)
=µv
KSIRO
X
k=1
pKSIRO
k
−c
KSIRO
X
k=1
pKSIRO
k
k
=λv
KSIRO
X
k=1
pKSIRO
k−1 −cλ
KSIRO
X
k=1
pKSIRO
k1
τk
=λ
KSIRO
X
k=1
pKSIRO
k−1 (v −cτk)
≥0,
where the third equation follows from the balance condition associated with the station-
arity of (first term) and Little’s law (second term), and the last inequality follows from
the fact that, for the buyer to enter a queue with k −1, v −cτk ≥0. Since (IRB) is
binding at K∗, it follows that K∗≥KSIRO.
D
Proof of Theorem 5
Proof. Recall that the first-best solution corresponds to a cutoff policy in which the
designer invites buyers into the queue if and only if the queue state is k < K∗(with
a possible rationing at k = K∗−1) and asks those invited to stay until they collect
v. Assume for ease of exposition that there is no rationing at k = K∗−1.D.41
The
optimal information policy, which, as noted before, informs buyers either “in” or “no
entry.” (The no entry is enforced with a commitment not to allocate the good in case of
entry.) Buyers, in turn, make an inference on the relevant history h ∈H0, based on the
designer’s mechanism, their recommended actions, as well as the amounts of time t ≥0
they have spent in the queue.
D.41The proof works more generally with a small adjustment in the updating formula; see Che and
Tercieux (forth.) for details.
61

Suppose a buyer has just arrived and receives the invitation to join the queue. What
does he believe about the queue length if he joins the queue? Given the optimal infor-
mation policy and the optimal cutoff K∗, Her rational belief at the steady state will be
that after joining the queue, its length will be k with probability:D.42
γ0
k =





pK∗
k−1
PK∗
i=1 pK∗
i−1 =
ρk−1
PK∗
i=1 ρi−1
if k = 1, ..., K∗
0
if k > K∗.
(D.4)
By Little’s law, the buyer’s expected wait time under the optimal information policy is
given by:
PK∗
k=1 kpK∗
k
λ PK∗
j=1 pK∗
j−1
,
independently of the service priority rule, if one were to stay in the queue until he collects
v. Hence, a buyer’s expected payoff from joining the queue under the latter assumption
is:
v −c
PK∗
k=1 kpK∗
k
λ PK∗
j=1 pK∗
j−1
=
1
λ PK∗
j=1 pK∗
j−1
"
λ
K∗
X
j=1
pK∗
j−1v −c
K∗
X
k=1
kpK∗
k
#
=
1
λ PK∗
j=1 pK∗
j−1
"
µ
K∗
X
k=1
pK∗
k v −c
K∗
X
k=1
kpK∗
k
#
∝
K∗
X
k=1
pK∗
k (µv −ck) ≥0,
where we use the balance condition to obtain the second equality and use (IRB) to
obtain the inequality. It follows that buyers have an incentive to join the queue (under
any service priority rule, including FCFS) when recommended by the optimal mechanism
under the optimal information policy, under the assumption that he will continue to stay
in the queue once he joins the queue.
Next, we show that a buyer has the incentive to stay in the queue once he has joined
D.42 This is simply a Bayesian update of the stationary distribution of the queue state conditional on
the recommendation to join (which indicates that k ∈{0, ..., K∗−1}). The use of stationarity means
that the buyer has a long-run belief that the system has been running for a long time. This is formally
verified as PASTA (“Poisson Arrival Sees Time Averages”); see Wolff (1982).
62

the queue under FCFS. To this end, we show that the residual expected wait time does
not increase in the amount of time t that the buyer spends in the queue. Under FCFS, a
sufficient statistic for the latter is a buyer’s queue position, ℓ, i.e., his arrival order within
the queue. At t = 0, his belief on queue position ℓ= 1, ..., K∗is simply given by γ0
ℓ. As
t increases, a buyer’s belief about his queue position evolves according to the recursion
equation: for any dt > 0,D.43
γt+dt
ℓ
= γt
ℓ(1 −µdt) + γt
ℓ+1µdt
PK∗
i=1 γt
i(1 −µdt)
+ o(dt),
for ℓ= 1, ..., K∗, where γt
K∗+1 ≡0. We wish to show that (γ0
ℓ) likelihood-ratio dominates
(γt
ℓ), for all t > 0, which will imply that the expected residual wait time decreases in t.
One can use the above equations (with dt →0) to derive a system of ordinary differ-
ential equations (ODEs) on the likelihood ratios:
˙rt
ℓ= µrt
ℓ
 rt
ℓ+1 −rt
ℓ

,
(D.5)
for ℓ= 2, ..., K∗, where rt
K∗+1 ≡0. The system has a unique solution by appealing to the
Banach fixed-point theorem. (D.4) yields boundary conditions: r0
ℓ= ρ for ℓ= 2, ..., K∗.
There are two cases. Suppose first K∗= ∞. In this case, rt
ℓis constant in t, so trivially
we have rt
ℓ≤r0
ℓfor all t.
Next, K∗< ∞. In this case, ˙r0
ℓ= 0 for all ℓ= 2, ..., K∗−1, and ˙r0
K∗< 0. Differenti-
ating the ODE once more, we get
¨rt
ℓ= µ ˙rt
ℓ
 rt
ℓ+1 −rt
ℓ

+ µrt
ℓ
 ˙rt
ℓ+1 −˙rt
ℓ

.
(D.6)
If ˙rt
ℓ> 0 at some t, there exists the smallest t at which ˙rt
ℓcrosses zero with ¨rt
ℓ> 0, for
D.43The numerator is the probability that his queue position is ℓafter staying in the queue for a length
t + dt of time. This event occurs if either (i) the buyer already has position ℓin the queue at t and none
of the agents ahead of him or himself have been served during dt; or (ii) if he has position ℓ+ 1 at t and
one buyer ahead of him is served by t + dt.
63

some ℓ.D.44 From (D.6), we have ¨rt
ℓ= 0, a contradiction.
D.44We choose the largest ℓif this happens for multiple ℓ’s.
64
