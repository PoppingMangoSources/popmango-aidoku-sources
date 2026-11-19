(() => {
	const fetch = window.fetch;
	window.fetch = async function (...args) {
		const response = await fetch.apply(this, args);
		const url = new URL(response.url || location.href);
		if (url.origin === location.origin && url.pathname === location.pathname.replace(/\/$/, "") + "/pages") {
			response.clone().text().then(body => { window.webnexPageList = body; });
		}
		return response;
	};
})();
