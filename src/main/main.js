
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { currentMonitor, getCurrentWindow, LogicalSize } from "@tauri-apps/api/window"
import { register, unregisterAll } from "@tauri-apps/plugin-global-shortcut"
import { pin, reset } from "tauri-plugin-wallpaper"

const isDev = import.meta.env.MODE === "development"
if (isDev) {
    unregisterAll()
    // invoke("devtools", { label: "main" })
}

const win = getCurrentWindow()

document.addEventListener("DOMContentLoaded", async () => {

    const leftStatsContainer = document.getElementById("left-stats-container")
    const rightStatsContainer = document.getElementById("right-stats-container")

    const fpsStatsSection = document.getElementById("fps-stats")
    const netStatsSection = document.getElementById("net-stats")
    const cpuStatsSection = document.getElementById("cpu-stats")
    const ramStatsSection = document.getElementById("ram-stats")
    const gpuStatsSection = document.getElementById("gpu-stats")

    const fpsAnimatedIcon = fpsStatsSection?.querySelector(".device-animated-icon")
    const netAnimatedIcon = netStatsSection?.querySelector(".device-animated-icon")
    const cpuAnimatedIcon = cpuStatsSection?.querySelector(".device-animated-icon")
    const ramAnimatedIcon = ramStatsSection?.querySelector(".device-animated-icon")
    const gpuAnimatedIcon = gpuStatsSection?.querySelector(".device-animated-icon")

    const fpsStatsData = fpsStatsSection?.getElementsByClassName("data")
    const netStatsData = netStatsSection?.getElementsByClassName("data")
    const cpuStatsData = cpuStatsSection?.getElementsByClassName("data")
    const ramStatsData = ramStatsSection?.getElementsByClassName("data")
    const gpuStatsData = gpuStatsSection?.getElementsByClassName("data")

    let isHiding
    let canExecuteShortcut = true

    // NETWORK
    invokeWithTimeout("start_network_speed_measurement")
    listen("on_network_speed_measured", async (e) => {

        const netStats = e.payload
        if (netStats) {

            const downloadSpeed = parseInt(netStats.net_download_speed)
            const uploadSpeed = parseInt(netStats.net_upload_speed)

            for (const data of Array.from(netStatsData)) {

                switch (data.id) {
                    case "net-download":
                        data.innerHTML = `${downloadSpeed || 0} <div>MB/s</div>`
                        break
                    case "net-upload":
                        data.innerHTML = `${uploadSpeed || 0} <div>MB/s</div>`
                        break
                }
            }

            const fastestChannel = Math.max(downloadSpeed, uploadSpeed)
            const netStep = Math.trunc(fastestChannel / 10) * 10
            const netFrameDuration = 150 - ((netStep * (150 - 50)) / 200)  //? proporzione inversa: max - ((val * (max - min)) / valmax)
            netAnimatedIcon.getAnimations()[0].updatePlaybackRate(150 / netFrameDuration)

            if (downloadSpeed <= 0 && uploadSpeed <= 0) {
                leftStatsContainer.setAttribute("hidden", "")
            } else {
                if (await win.isVisible() && !isHiding) leftStatsContainer.removeAttribute("hidden")
            }
        }
    })

    // FPS
    invokeWithTimeout("start_fps_measurement")
    listen("on_fps_measured", (e) => {

        const fps = parseInt(e.payload) || 0

        for (const data of Array.from(fpsStatsData)) {

            switch (data.id) {
                case "fps-count":
                    data.textContent = fps
                    break
            }
        }

        const fpsStep = Math.trunc(fps / 10) * 10
        const fpsFrameDuration = 150 - ((fpsStep * (150 - 50)) / 200)  //? proporzione inversa: max - ((val * (max - min)) / valmax)
        fpsAnimatedIcon.getAnimations()[0].updatePlaybackRate(150 / fpsFrameDuration)
    })

    createRecursiveLoop(async () => {

        // CPU
        const cpuStats = await invokeWithTimeout("get_cpu_stats")
        if (cpuStats) {

            for (const data of Array.from(cpuStatsData)) {

                switch (data.id) {
                    case "cpu-usage":
                        data.innerHTML = `${cpuStats.cpu_usage || 0} <div>%</div>`
                        break
                }
            }

            const cpuPercStep = Math.trunc(cpuStats.cpu_usage / 10) * 10
            const cpuFrameDuration = 150 - ((cpuPercStep * (150 - 50)) / 100)  //? proporzione inversa: max - ((perc% * (max - min)) / 100%)
            cpuAnimatedIcon.getAnimations()[0].updatePlaybackRate(150 / cpuFrameDuration)
        }


        // RAM
        const ramStats = await invokeWithTimeout("get_ram_stats")
        if (ramStats) {

            for (const data of Array.from(ramStatsData)) {

                switch (data.id) {
                    case "ram":
                        data.innerHTML = `${ramStats.ram || 0} <div>%</div>`
                        break
                }
            }

            const ramPercStep = Math.trunc(ramStats.ram / 10) * 10
            const ramFrameDuration = 150 - ((ramPercStep * (150 - 20)) / 100)  //? proporzione inversa: max - ((perc% * (max - min)) / 100%)
            ramAnimatedIcon.getAnimations()[0].updatePlaybackRate(150 / ramFrameDuration)
        }


        // GPU
        const gpuStats = await invokeWithTimeout("get_gpu_stats")
        if (gpuStats) {

            for (const data of Array.from(gpuStatsData)) {

                switch (data.id) {
                    case "gpu-memory":
                        data.innerHTML = `${gpuStats.gpu_memory || 0} <div>%</div>`
                        break
                    case "gpu-temp":
                        data.innerHTML = `${gpuStats.gpu_temp || 0} <div>°C</div>`
                        break
                    case "gpu-fan-speed-0":
                        data.setAttribute("fan-available", gpuStats.gpu_fan_speed_0 > 0)
                        data.innerHTML = `${gpuStats.gpu_fan_speed_0 || 0} <div>%</div>`
                        break
                    case "gpu-fan-speed-1":
                        data.setAttribute("fan-available", gpuStats.gpu_fan_speed_1 > 0)
                        data.innerHTML = `${gpuStats.gpu_fan_speed_1 || 0} <div>%</div>`
                        break
                    case "gpu-fan-speed-2":
                        data.setAttribute("fan-available", gpuStats.gpu_fan_speed_2 > 0)
                        data.innerHTML = `${gpuStats.gpu_fan_speed_2 || 0} <div>%</div>`
                        break
                }
            }

            const fastestFan = Math.max(gpuStats.gpu_fan_speed_0, gpuStats.gpu_fan_speed_1, gpuStats.gpu_fan_speed_2)
            const gpuPercStep = Math.trunc(fastestFan / 10) * 10
            const gpuFrameDuration = 50 - ((gpuPercStep * (50 - 5)) / 100)  //? proporzione inversa: max - ((perc% * (max - min)) / 100%)
            gpuAnimatedIcon.getAnimations()[0].updatePlaybackRate(50 / gpuFrameDuration)
        }

    }, 1000)

    // gestione finestra
    const screenWidth = (await currentMonitor()).workArea.size.width
    await win.setSize(new LogicalSize(screenWidth, 50))
    await win.setIgnoreCursorEvents(true)

    await reset(win.label)
    await pin(win.label)

    //? fix primo sliding
    await win.show()
    await win.hide()

    // registrazione shortcut
    await register("Super+Z", async (e) => {

        if (e.state === "Pressed" && canExecuteShortcut) {
            canExecuteShortcut = false

            if (!(await win.isVisible())) {

                //? ricalcola dimensione ogni volta
                const screenWidth = (await currentMonitor()).workArea.size.width
                await win.setSize(new LogicalSize(screenWidth, 50))

                await win.show()
                rightStatsContainer.removeAttribute("hidden")

            } else {
                isHiding = true
                leftStatsContainer.setAttribute("hidden", "")
                rightStatsContainer.setAttribute("hidden", "")

                await new Promise(res => setTimeout(res, 400))
                await win.hide()
                isHiding = false
            }

            setTimeout(() => canExecuteShortcut = true, 400)
        }
    })
})


async function invokeWithTimeout(command, args = {}) {
    return Promise.race([
        invoke(command, args),
        new Promise(res => setTimeout(() => res({}), 5000))
    ])
}

async function createRecursiveLoop(callback, delay = 0) {

    const shouldStop = await callback()
    if (shouldStop) return

    setTimeout(() => createRecursiveLoop(callback, delay), delay)
}

function log(...text) {
    console.log(...text)
    invoke("log", { logs: text })
}

