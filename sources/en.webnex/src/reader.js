endpoint => {
	const url = new URL(endpoint, location.href);
	const fail = message => { window.webnexPageList = JSON.stringify({ message }); };
	if (url.origin !== location.origin) {
		fail("WebNex returned a page list on a different origin.");
		return "";
	}

	const controller = new AbortController();
	const timer = setTimeout(() => controller.abort(), 12000);
	fetch(url, { signal: controller.signal }).then(async response => {
		if (!response.ok) {
			const error = await response.json().catch(() => null);
			const detail = typeof error?.message === "string" ? ` ${error.message}` : "";
			fail(`WebNex returned HTTP ${response.status}.${detail}`);
			return;
		}
		window.webnexPageList = await response.text();
	}).catch(() => fail("WebNex could not fetch the chapter's page list. Open the chapter on webnex.cc."))
		.finally(() => clearTimeout(timer));
	return "";
}
