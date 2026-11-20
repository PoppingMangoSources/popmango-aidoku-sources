(async () => {
	if (location.origin !== "https://webnex.cc" && location.origin !== "https://www.webnex.cc") return;

	const fail = message => { window.webnexPageList = JSON.stringify({ message }); };
	const findReader = value => {
		if (Array.isArray(value)) {
			for (const child of value) {
				const reader = findReader(child);
				if (reader) return reader;
			}
		} else if (value?.chapter && value.series && typeof value.pagesUrl === "string" && value.pagesUrl) {
			return value.pagesUrl;
		}
	};

	// Next.js can split one Flight row across several inline scripts.
	const chunks = [];
	for (const script of document.scripts) {
		const match = script.textContent.trim().match(/^self\.__next_f\.push\((\[1,.*\])\);?$/s);
		if (match) {
			try {
				const [, chunk] = JSON.parse(match[1]);
				if (typeof chunk === "string") chunks.push(chunk);
			} catch {}
		}
	}
	let endpoint;
	for (const row of chunks.join("").split("\n")) {
		if (!row.includes('"pagesUrl"') || !/^[0-9a-f]+:/i.test(row)) continue;
		try {
			endpoint = findReader(JSON.parse(row.slice(row.indexOf(":") + 1)));
			if (endpoint) break;
		} catch {}
	}
	if (!endpoint) {
		const notice = document.querySelector("section.state .notice-error, body > pre")?.textContent.trim();
		fail(notice || "WebNex did not provide the chapter's page list. Open it on webnex.cc.");
		return;
	}

	const controller = new AbortController();
	const timer = setTimeout(() => controller.abort(), 12000);
	try {
		const url = new URL(endpoint, location.href);
		if (url.origin !== location.origin) {
			fail("WebNex returned a page list on a different origin.");
			return;
		}
		const response = await fetch(url, { signal: controller.signal });
		if (response.ok) {
			window.webnexPageList = await response.text();
		} else {
			const error = await response.json().catch(() => null);
			const detail = typeof error?.message === "string" ? ` ${error.message}` : "";
			fail(`WebNex returned HTTP ${response.status}.${detail}`);
		}
	} catch {
		fail("WebNex could not fetch the chapter's page list. Open the chapter on webnex.cc.");
	} finally {
		clearTimeout(timer);
	}
})();
