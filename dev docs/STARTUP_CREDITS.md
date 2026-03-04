# STARTUP CLOUD CREDIT PROGRAMS

**Last updated:** 2026-03-04
**Status:** Not yet applied — both basic tracks ready to submit
**Priority:** Apply basic tracks immediately ($6K available, 15–30 min effort)

---

## 1. Overview

Two major cloud credit programs are available to Yapper right now. Both have a low-effort basic
track ($1K–$5K, apply today) and a high-value investor track ($100K–$150K, requires VC/accelerator
affiliation). Even without investor backing, the basic track credits cover ~2 years of Tier 1
infrastructure costs.

| Program | Basic Track | Investor Track | Approval Time |
|---------|-------------|---------------|---------------|
| **AWS Activate** | $1,000 | Up to $100,000 | 7–10 business days |
| **Microsoft for Startups** | $1,000–$5,000 | Up to $150,000 | 3 business days |
| **Combined (basic)** | **~$6,000** | | |
| **Combined (with investor)** | | **~$250,000** | |

> **Reference:** See `INFRASTRUCTURE.md` for how credits map to infrastructure tiers.
> $6K basic credits ≈ 2 years of Tier 1 costs (~$3/month). $250K with investor ≈ infrastructure
> through Series A.

---

## 2. Yapper Eligibility Checklist

Run through this before applying. All items must be ✅ for the application to succeed.

### Both Programs

- [x] Software product (not a consultancy, agency, or licensed product)
- [x] Privately held, for-profit
- [x] Not Series C or beyond (bootstrapped/pre-seed)
- [x] Company website exists — yapperhq.com
- [ ] **Business email** — must use `@yapperhq.com`, not Gmail (set up via Cloudflare Email Routing)
- [ ] **LinkedIn company page** for Yapper — create before applying

### AWS Activate Only

- [ ] AWS account created with a **payment method added** (must be on paid tier, not just free tier)
- [x] Founded within the last 10 years

### Microsoft for Startups Only

- [ ] **Demo video** recorded (Loom or Vimeo, max 10 minutes, showing website + working MVP)
- [x] Azure services available in South Africa ✅

---

## 3. What to Prepare (Do This First)

Complete these once — reused across both applications and all future investor/accelerator outreach.

### 3a. Business Email

Set up `yourname@yapperhq.com` via Cloudflare Email Routing (free):
1. Cloudflare Dashboard → yapperhq.com → Email → Email Routing
2. Add a custom address → forward to your personal email
3. Use this address for all applications

### 3b. One-Paragraph Pitch

> Yapper is a real-time end-to-end encrypted chat platform built for Gen Z, combining the
> familiarity of Discord-style servers and channels with Signal-grade E2EE privacy, built-in
> parental controls for under-18 users, and native audio/video yap formats. Built on Rust +
> SvelteKit with a zero-compromise security model — no plaintext ever touches the server.
> Currently in MVP, targeting iOS, Android, Web, and Desktop from a single codebase.

### 3c. Demo Video Script (10 minutes max)

Structure:
1. **0:00–1:00** — Show yapperhq.com marketing site + waitlist
2. **1:00–3:00** — Sign up, register, verify email
3. **3:00–5:00** — Send an E2EE DM, show message is encrypted in transit (open DevTools → Network)
4. **5:00–7:00** — Create a server, join a channel, send a group message
5. **7:00–9:00** — Show parental dashboard, pending alerts, child setup wizard
6. **9:00–10:00** — Brief architecture slide: Rust backend, SvelteKit frontend, Signal Protocol E2EE

Record with Loom (free) and set to public link.

### 3d. LinkedIn Company Page

Create at linkedin.com/company/create:
- Company name: Yapper
- Website: yapperhq.com
- Industry: Software Development
- Company size: 1–10 employees
- Description: Use the pitch paragraph above

---

## 4. AWS Activate

### Track 1 — Founders Package ($1,000)

**Apply at:** [aws.amazon.com/startups/credits](https://aws.amazon.com/startups/credits)

#### Pre-requisites
- [ ] AWS account created
- [ ] Payment method added to AWS account (required — must be on paid tier)
- [ ] Business email ready

#### Application Form Fields

| Field | What to Enter |
|-------|--------------|
| Company name | Yapper |
| Website | https://yapperhq.com |
| AWS Account ID | (from AWS Console → top right) |
| Product description | Use pitch paragraph above |
| Funding stage | Bootstrapped / Pre-Seed |
| Founded date | 2025 |
| Primary use case | Web & Mobile Application |

#### After Approval
- Credits applied to AWS account within 7–10 business days
- Credits valid for **2 years** from activation
- Can be used for any AWS service (EC2, RDS, S3, CloudFront, SES, etc.)

---

### Track 2 — Portfolio Package ($100,000)

**Requires:** An Activate Provider Organizational ID from a registered VC, accelerator, or incubator.

#### How to Get a Provider ID Without Traditional VC Backing

| Route | Effort | Notes |
|-------|--------|-------|
| **Stripe Atlas** (incorporate via Stripe) | Low | Stripe is an AWS Activate Provider; incorporates your company and issues an Org ID |
| **Mercury** or **Brex** (startup bank account) | Low | Some banking providers have AWS provider partnerships |
| **Silicon Cape** (SA startup hub) | Medium | Cape Town–based; check if registered AWS provider |
| **Bandwidth Barn** (SA incubator) | Medium | Cape Town–based incubator; may be registered |
| **AlphaCode** (Rand Merchant) | Medium | SA fintech/tech incubator |
| Join any registered accelerator | Varies | Full list: [aws.amazon.com/activate/portfolio-detail](https://aws.amazon.com/activate/portfolio-detail) |

#### Application Form Fields (in addition to Track 1)

| Field | What to Enter |
|-------|--------------|
| Provider Organizational ID | (from your provider) |
| Most recent funding round | Bootstrapped (or pre-seed amount if applicable) |
| Funding date | N/A or actual date |
| Previously received AWS credits? | No (or enter previous amount if any) |

---

## 5. Microsoft for Startups (Founders Hub)

### Track 1 — Basic ($1,000 → up to $5,000)

**Apply at:** [portal.startups.microsoft.com](https://portal.startups.microsoft.com)

#### Application Steps

1. Go to portal.startups.microsoft.com → **Get started now**
2. Sign in with a **Microsoft account** linked to `@yapperhq.com` (not personal email)
3. **Authenticate via LinkedIn** — must have a LinkedIn profile
4. Select startup stage: **Building MVP**
5. Enter company details (see table below)
6. Upload or link demo video (Loom URL)
7. Submit — decision within **3 business days**

#### Application Form Fields

| Field | What to Enter |
|-------|--------------|
| Company name | Yapper |
| Website | https://yapperhq.com |
| Business email | yourname@yapperhq.com |
| Startup stage | Building MVP |
| Product description | Use pitch paragraph above |
| Industry | Software / Communication |
| Target market | Consumer / Gen Z / Family Safety |
| Demo video | Loom link (see Section 3c) |

#### Credit Unlock Path

```
Apply → Approved → $1,000 credits immediately
  └─ Verify business (LinkedIn company page + domain match)
       └─ Unlock up to $5,000 total
```

#### After Approval

- Azure credits valid for **12 months**
- Access to GitHub Copilot (free while in program)
- Access to Microsoft 365 (limited seats)
- Azure OpenAI Service access
- Optional: Microsoft mentorship sessions

---

### Track 2 — Investor Offer ($150,000)

**Requires:** Referral code from an investor in the Microsoft for Startups Investor Network.

#### How to Get a Referral

| Route | Effort | Notes |
|-------|--------|-------|
| **Y Combinator** (apply to YC) | High | YC is in the network; acceptance = referral |
| **Local SA angel investors** | Medium | SA angel networks may be registered |
| **Microsoft AI for Good** | Medium | Microsoft's own programs can bridge you in |
| **Ask your accountant/lawyer** | Low | Some startup advisors have referral relationships |
| **LinkedIn outreach to investors** | Medium | Ask if they're in the Microsoft for Startups Investor Network |

---

## 6. Credit Utilisation Strategy

If basic track ($6K total) is approved, allocate as follows:

| Service | Allocation | Months Coverage |
|---------|-----------|----------------|
| AWS S3 + Glacier IR (media storage) | $1,500 | ~3–4 years at Tier 1 rates |
| AWS CloudFront (CDN) | $500 | ~2 years at Tier 1 traffic |
| Azure PostgreSQL (if testing Azure) | $1,000 | ~7 months |
| Azure Container Apps (if testing Azure) | $1,000 | ~7 months |
| Buffer / overflow | $2,000 | Covers unexpected usage spikes |

If investor track ($250K) is approved, allocate as follows:

| Service | Allocation | Purpose |
|---------|-----------|---------|
| AWS RDS reserved (1yr) | $5,000 | Pre-pay reserved instances for Tier 2–3 DB |
| AWS EC2 reserved (1yr) | $8,000 | Pre-pay reserved compute |
| AWS S3 + Glacier + CloudFront | $10,000 | 2–3 years of media storage |
| Azure infrastructure (testing + staging) | $20,000 | Run staging env on Azure for comparison |
| Buffer | Remainder | Growth runway |

---

## 7. Additional Credit Programs to Explore

Don't stop at AWS and Azure:

| Program | Credits | Notes |
|---------|---------|-------|
| **Google for Startups** | Up to $200,000 | Largest credit pool; includes GCP, Firebase, Google Workspace |
| **GitHub for Startups** | Free GitHub Team (12 mo) | Requires VC/accelerator referral |
| **Cloudflare for Startups** | $250/mo Cloudflare credits | Useful for Workers, R2, Pages Pro |
| **Vercel for Startups** | Free Pro plan | If ever considering Vercel for frontend |
| **Sentry for Startups** | Free Team plan | Error monitoring upgrade |
| **Fly.io (no formal program)** | N/A | No startup credit program; already free |
| **Neon (no formal program)** | N/A | No startup credit program; already on free tier |

**Google for Startups** is worth applying to immediately after AWS/Azure — $200K in GCP credits
covers Firebase, Cloud Run, and Cloud SQL if needed, and the Google Workspace credits reduce
operational costs for the team.

Apply at: [cloud.google.com/startup](https://cloud.google.com/startup)

---

## 8. Application Tracker

Track status here as applications are submitted:

| Program | Track | Applied | Credits | Expiry | Status |
|---------|-------|---------|---------|--------|--------|
| AWS Activate | Founders ($1K) | [ ] | — | — | Not started |
| AWS Activate | Portfolio ($100K) | [ ] | — | — | Needs provider ID |
| Microsoft for Startups | Basic ($5K) | [ ] | — | — | Not started |
| Microsoft for Startups | Investor ($150K) | [ ] | — | — | Needs investor referral |
| Google for Startups | Basic ($200K) | [ ] | — | — | Not started |
| GitHub for Startups | Free Team | [ ] | — | — | Not started |
| Cloudflare for Startups | $250/mo | [ ] | — | — | Not started |

---

*Cross-reference: `INFRASTRUCTURE.md` — infrastructure tiers and cost projections*
*Cross-reference: `HANDOVER.md` Section 10 — deployment and secrets management*
