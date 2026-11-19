(() => {
	if (window.webnexPageList) return window.webnexPageList;
	const denied = document.contentType === "text/plain" && document.body?.textContent.trim();
	const notice = document.querySelector("section.state .notice-error")?.textContent.trim();
	return denied || notice ? JSON.stringify({ message: denied || notice }) : "";
})()
