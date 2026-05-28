<div class="container">
	<header class="title">
		<h1>金欠<span class="kana">きんけつ</span></h1>
		<p class="tagline">サブスクの全貌、メールから。</p>
	</header>

	<section class="grid">
		<article class="glass card">
			<h2>月額合計</h2>
			<p class="big">¥ —</p>
			<p class="muted">connect Gmail to discover</p>
		</article>
		<article class="glass card">
			<h2>このあと請求</h2>
			<p class="muted">no subscriptions yet</p>
		</article>
		<article class="glass card span-2">
			<h2>支払元別</h2>
			<p class="muted">add a payment method to see the breakdown</p>
		</article>
	</section>
</div>

<style>
	.container {
		max-width: 1024px;
		margin: 0 auto;
		padding: 4rem 2rem;
	}
	.title h1 {
		font-size: clamp(2.5rem, 4vw, 4rem);
		font-weight: 800;
		margin: 0;
		letter-spacing: -0.02em;
	}
	.title .kana {
		font-size: 0.4em;
		font-weight: 500;
		margin-left: 0.5em;
		opacity: 0.5;
	}
	.title .tagline {
		margin: 0.5rem 0 3rem;
		color: var(--kk-text-muted);
		font-size: 1.1rem;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1rem;
	}
	.card {
		padding: 1.5rem;
		border-radius: var(--kk-radius-md);
	}
	.card h2 {
		margin: 0 0 0.75rem;
		font-size: 0.85rem;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--kk-text-muted);
		font-weight: 500;
	}
	.card .big {
		margin: 0;
		font-size: 2.5rem;
		font-weight: 700;
		letter-spacing: -0.02em;
	}
	.card .muted {
		margin: 0;
		color: var(--kk-text-muted);
	}
	.card.span-2 {
		grid-column: span 2;
	}
	@media (max-width: 640px) {
		.grid {
			grid-template-columns: 1fr;
		}
		.card.span-2 {
			grid-column: span 1;
		}
	}
</style>
