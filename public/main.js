document.addEventListener("DOMContentLoaded", () => {
	const REPO_URL = document.getElementById("repo-url").textContent.trim();
	const isApple = /iphone|ipad|ipados|macintosh|mac os x/i.test(navigator.userAgent);
	const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

	const langMap = {
		en: "English", es: "Spanish", fr: "French", de: "German", ja: "Japanese",
		zh: "Chinese", ru: "Russian", it: "Italian", ko: "Korean", pt: "Portuguese",
		id: "Indonesian", th: "Thai", vi: "Vietnamese", tr: "Turkish", pl: "Polish",
		ar: "Arabic", hi: "Hindi",
	};
	const labelToCode = Object.fromEntries(Object.entries(langMap).map(([c, l]) => [l, c]));
	const languageLabel = (languages) => {
		if (!Array.isArray(languages) || languages.length === 0) return "Unknown";
		if (languages.length > 1 || languages.includes("multi")) return "Multi-Language";
		return langMap[languages[0]] || languages[0];
	};

	// The icon shows the current theme: moon in dark mode, sun in light mode.
	const toggle = document.getElementById("theme-toggle");
	const root = document.documentElement;
	const labelToggle = () =>
		toggle.setAttribute("aria-label", root.dataset.theme === "dark" ? "Switch to light mode" : "Switch to dark mode");
	labelToggle();
	toggle.addEventListener("click", () => {
		root.dataset.theme = root.dataset.theme === "dark" ? "light" : "dark";
		try {
			localStorage.setItem("theme", root.dataset.theme);
		} catch {}
		labelToggle();
	});

	// System toasts
	const toasts = document.getElementById("toasts");
	function toast(text) {
		const el = document.createElement("div");
		el.className = "toast";
		el.innerHTML = '<span class="toast__tag">SYSTEM</span>';
		el.append(text);
		toasts.appendChild(el);
		setTimeout(() => el.classList.add("toast--out"), 3600);
		setTimeout(() => el.remove(), 4200);
	}

	// Affection: five hearts, 20 points each, remembered in this browser when storage allows
	let affection = 0;
	try {
		affection = Math.min(100, Number(localStorage.getItem("affection")) || 0);
	} catch {}
	const affectionEl = document.getElementById("affection");
	function renderAffection() {
		const full = Math.min(5, Math.floor(affection / 20));
		affectionEl.textContent = "♥".repeat(full) + "♡".repeat(5 - full);
		affectionEl.setAttribute("aria-label", "Affection " + full + " of 5");
	}
	function gain(points, reason) {
		const before = Math.floor(affection / 20);
		affection = Math.min(100, affection + points);
		try {
			localStorage.setItem("affection", String(affection));
		} catch {}
		renderAffection();
		affectionEl.classList.remove("affection--pop");
		void affectionEl.offsetWidth;
		affectionEl.classList.add("affection--pop");
		toast(reason + " Affection +" + points + ".");
		if (before < 5 && affection >= 100) setTimeout(() => toast("True ending unlocked. The villainess lives!"), 900);
	}
	renderAffection();

	// Visual novel dialogue
	const lines = [
		"Ah! A reader! You came all this way with... zero sources?",
		"The System says the villainess dies in chapter 1 unless someone gives her something to read.",
		"I-it's not like I ported these for you or anything. Just pick one, okay?",
	];
	const lineEl = document.getElementById("line");
	const nextBtn = document.getElementById("next");
	const choices = document.getElementById("choices");
	let index = 0;
	let typing = null;
	function showLine(i) {
		clearInterval(typing);
		const text = lines[i];
		if (reduceMotion) {
			lineEl.textContent = text;
			return;
		}
		let n = 0;
		lineEl.textContent = "";
		typing = setInterval(() => {
			lineEl.textContent = text.slice(0, ++n);
			if (n >= text.length) {
				clearInterval(typing);
				typing = null;
			}
		}, 28);
	}
	function advance() {
		if (typing) {
			clearInterval(typing);
			typing = null;
			lineEl.textContent = lines[index];
			return;
		}
		if (index < lines.length - 1) {
			showLine(++index);
		}
		if (index === lines.length - 1) {
			choices.hidden = false;
			nextBtn.hidden = true;
		}
	}
	nextBtn.addEventListener("click", advance);
	document.querySelector(".dialogue").addEventListener("click", (e) => {
		if (e.target !== nextBtn) advance();
	});
	showLine(0);

	async function copyUrl() {
		try {
			await navigator.clipboard.writeText(REPO_URL);
			gain(10, "Repository URL copied.");
		} catch {
			toast("Couldn't copy. Select the URL in the quest window instead.");
		}
	}
	document.getElementById("choice-copy").addEventListener("click", copyUrl);
	document.getElementById("copy-url").addEventListener("click", copyUrl);
	for (const link of document.querySelectorAll(".add-repo")) {
		link.addEventListener("click", (e) => {
			if (!isApple) {
				e.preventDefault();
				toast("Open this page on your iPhone or iPad, or paste the URL in Aidoku under Settings → Source Lists.");
				return;
			}
			gain(40, "Quest accepted.");
		});
	}
	setTimeout(() => toast("New quest: Save the villainess."), 900);

	// Routes
	const list = document.getElementById("source-list");
	const search = document.getElementById("source-search");
	const languageSelect = document.getElementById("language-select");
	const ratingSelect = document.getElementById("rating-select");
	const count = document.getElementById("total-count");
	let cards = [];

	const heat = (rating) => (rating === 2 ? "18+" : rating === 1 ? "17+" : "Safe");

	function matches(source) {
		const lang = languageSelect.value;
		const label = languageLabel(source.languages);
		const langOk =
			!lang ||
			label === lang ||
			(lang !== "Multi-Language" &&
				label === "Multi-Language" &&
				Array.isArray(source.languages) &&
				source.languages.includes(labelToCode[lang]));

		const rating = ratingSelect.value;
		const r = source.contentRating || 0;
		const ratingOk =
			!rating || (rating === "safe" && r === 0) || (rating === "contains-nsfw" && r === 1) || (rating === "nsfw" && r === 2);

		const q = search.value.trim().toLowerCase();
		const queryOk =
			!q ||
			[source.name, source.baseURL, ...(Array.isArray(source.altNames) ? source.altNames : [])].some(
				(s) => s && s.toLowerCase().includes(q),
			);
		return langOk && ratingOk && queryOk;
	}

	function filter() {
		let visible = 0;
		for (const { source, card } of cards) {
			const show = matches(source);
			card.hidden = !show;
			if (show) visible++;
		}
		const total = cards.length;
		document.getElementById("lv-fill").style.width = (total ? (visible / total) * 100 : 0) + "%";
		count.textContent = visible + " of " + total + " routes";
		let empty = list.querySelector(".empty");
		if (visible === 0 && total > 0) {
			if (!empty) {
				empty = document.createElement("div");
				empty.className = "empty";
				empty.innerHTML =
					"<b>BAD ENDING</b><span>No route matches. The villainess remains doomed.</span>" +
					'<button class="button" type="button">Load last save</button>';
				empty.querySelector("button").addEventListener("click", () => {
					search.value = "";
					languageSelect.value = "";
					ratingSelect.value = "";
					filter();
				});
				list.appendChild(empty);
			}
		} else if (empty) {
			empty.remove();
		}
	}
	search.addEventListener("input", filter);
	languageSelect.addEventListener("change", filter);
	ratingSelect.addEventListener("change", filter);

	function icon(source, className) {
		const img = document.createElement("img");
		img.className = className;
		img.src = source.iconURL;
		img.alt = "";
		img.loading = "lazy";
		return img;
	}

	function card(source) {
		const el = document.createElement("article");
		el.className = "route";

		const top = document.createElement("div");
		top.className = "route__top";
		if (source.iconURL) top.appendChild(icon(source, "route__icon"));
		const badge = document.createElement("span");
		badge.className = "route__heat route__heat--" + (source.contentRating || 0);
		badge.textContent = heat(source.contentRating);
		top.appendChild(badge);

		const name = document.createElement("h3");
		name.textContent = source.name;
		const meta = document.createElement("p");
		meta.className = "route__meta";
		meta.textContent = languageLabel(source.languages) + " · v" + source.version;
		const url = document.createElement("p");
		url.className = "route__url";
		url.textContent = (source.baseURL || "").replace(/^https?:\/\/(www\.)?/, "");

		const get = document.createElement("a");
		get.className = "route__get";
		get.href = source.downloadURL;
		get.setAttribute("download", "");
		get.textContent = "Choose route ♥";
		get.addEventListener("click", () => gain(5, "Route “" + source.name + "” saved."));

		el.append(top, name, meta, url, get);
		return el;
	}

	fetch("index.min.json")
		.then((response) => {
			if (!response.ok) throw new Error("Failed to load sources");
			return response.json();
		})
		.then((data) => {
			const sources = (data.sources || []).slice().sort((a, b) => a.name.localeCompare(b.name));
			if (sources.length === 0) {
				list.innerHTML = '<p class="empty">No routes yet. The villainess waits.</p>';
				return;
			}

			const labels = [...new Set(sources.map((s) => languageLabel(s.languages)))].sort();
			for (const label of labels) languageSelect.add(new Option(label, label));

			cards = sources.map((source) => ({ source, card: card(source) }));
			list.replaceChildren(...cards.map((c) => c.card));

			document.getElementById("lv").textContent = "LV. " + sources.length;
			document.getElementById("reward").textContent = sources.length + " routes unlocked";

			// Two identical sets: the track slides by one set's width, so the loop has no seam.
			const ticker = document.getElementById("ticker");
			const set = document.createElement("div");
			set.className = "ticker__set";
			for (const s of sources.filter((s) => s.iconURL)) {
				const item = document.createElement("span");
				item.className = "ticker__item";
				const img = icon(s, "ticker__icon");
				img.alt = s.name;
				img.loading = "eager";
				item.appendChild(img);
				set.appendChild(item);
			}
			const copy = set.cloneNode(true);
			copy.setAttribute("aria-hidden", "true");
			ticker.append(set, copy);
			filter();
		})
		.catch(() => {
			list.innerHTML = '<p class="empty">The System failed to load the routes. Try again in a moment.</p>';
		});
});
