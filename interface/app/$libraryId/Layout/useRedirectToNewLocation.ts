import { explorerStore } from '@sd/explorer';
import { useZodRouteParams } from '@sd/interface-core';
import { useNavigate } from 'react-router';
import { useLibraryQuery, useSelector } from '@sd/client';

import { LibraryIdParamsSchema } from '../../route-schemas';

/**
 * When a user adds a location and checks the should redirect box,
 * this hook will redirect them to the location
 * once the indexer has been invoked
 */

export const useRedirectToNewLocation = () => {
	const navigate = useNavigate();
	const { libraryId } = useZodRouteParams(LibraryIdParamsSchema);
	const newLocation = useSelector(explorerStore, (s) => s.newLocationToRedirect);
	const { data: jobGroups } = useLibraryQuery(['jobs.reports'], {
		enabled: newLocation != null,
		refetchOnWindowFocus: false
	});

	const hasIndexerJob = jobGroups
		?.flatMap((j) => j.jobs)
		.some(
			(j) =>
				j.name === 'indexer' &&
				(j.metadata as any)?.location.id === newLocation &&
				(j.completed_task_count > 0 || j.completed_at != null)
		);

	if (hasIndexerJob) {
		navigate(`/${libraryId}/location/${newLocation}`);
		explorerStore.newLocationToRedirect = null;
	}
};