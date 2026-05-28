<div class="blobs" aria-hidden="true">
	<div class="blob blob-mochi"></div>
	<div class="blob blob-yuzu"></div>
	<div class="blob blob-matcha"></div>
	<div class="blob blob-sora"></div>
	<div class="blob blob-fuji"></div>
</div>

<style>
	.blobs {
		position: absolute;
		inset: 0;
		overflow: hidden;
		pointer-events: none;
		filter: blur(80px);
		opacity: 0.55;
	}
	.blob {
		position: absolute;
		width: 40vmax;
		height: 40vmax;
		border-radius: 50%;
		will-change: transform;
	}
	.blob-mochi {
		background: var(--color-accent-mochi);
		top: -10%;
		left: -5%;
		animation: drift1 28s ease-in-out infinite alternate;
	}
	.blob-yuzu {
		background: var(--color-accent-yuzu);
		top: 20%;
		right: -10%;
		animation: drift2 36s ease-in-out infinite alternate;
	}
	.blob-matcha {
		background: var(--color-accent-matcha);
		bottom: -10%;
		left: 30%;
		animation: drift3 32s ease-in-out infinite alternate;
	}
	.blob-sora {
		background: var(--color-accent-sora);
		top: 50%;
		left: -15%;
		animation: drift1 40s ease-in-out infinite alternate-reverse;
	}
	.blob-fuji {
		background: var(--color-accent-fuji);
		bottom: 5%;
		right: 10%;
		animation: drift2 44s ease-in-out infinite alternate-reverse;
	}
	@keyframes drift1 {
		from {
			transform: translate3d(0, 0, 0) scale(1);
		}
		to {
			transform: translate3d(8vw, 6vh, 0) scale(1.15);
		}
	}
	@keyframes drift2 {
		from {
			transform: translate3d(0, 0, 0) scale(1);
		}
		to {
			transform: translate3d(-10vw, 8vh, 0) scale(0.9);
		}
	}
	@keyframes drift3 {
		from {
			transform: translate3d(0, 0, 0) scale(0.95);
		}
		to {
			transform: translate3d(6vw, -10vh, 0) scale(1.1);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.blob {
			animation: none;
		}
	}
</style>
