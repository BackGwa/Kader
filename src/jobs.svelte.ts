import { listen } from "@tauri-apps/api/event";
import { snapshot, type Job } from "./api";

class Jobs {
  list = $state<Job[]>([]);

  async connect() {
    const stopList = await listen<Job[]>("jobs", (event) => {
      this.list = event.payload;
    });
    const stopJob = await listen<Job>("job", (event) => {
      const index = this.list.findIndex((job) => job.id === event.payload.id);
      if (index >= 0) {
        this.list[index] = event.payload;
      }
    });
    this.list = await snapshot();
    return () => {
      stopList();
      stopJob();
    };
  }
}

export const jobs = new Jobs();
